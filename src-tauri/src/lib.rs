use std::sync::Mutex;
use lazy_static::lazy_static;
use tokio_postgres::{NoTls, Error};
use serde::Deserialize;
use std::collections::HashMap;
use scraper::{Html, Selector};
use tms_service;
use std::time::Duration;
use tokio::time::sleep;

lazy_static! {
    static ref COOKIE_STORAGE: Mutex<Option<String>> = Mutex::new(None);
    static ref USERNAME_STORAGE: Mutex<Option<String>> = Mutex::new(None);
}

#[derive(serde::Serialize)]
struct LoginResult {
    success: bool,
    message: String,
}

#[derive(Deserialize, Debug)]
struct ApiLoginResponse {
    msg: String,
    // a_id: i32,
    // username: String,
    // code: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct WorkOrder {
    gdid: String,
    n2n: String,
    messageq: String,
    messagea: Option<String>,
    iscreate: i32,
    isfeedback: i32,
    isclose: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct WorkOrderResponse {
    orders: Vec<WorkOrder>,
    total: i32,
}

#[tauri::command]
async fn login(username: String, password: String) -> Result<LoginResult, String> {
    // First, authenticate against the external API
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();
    let mut params = HashMap::new();
    params.insert("username", username.clone());
    params.insert("password", password);

    let res = client.post("https://c.baobaot.com/user/ajax_login")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("API请求失败: {}", e))?;

    if res.status().is_success() {
        if let Some(cookie) = res.headers().get("set-cookie") {
            if let Ok(cookie_str) = cookie.to_str() {
                let mut storage = COOKIE_STORAGE.lock().unwrap();
                *storage = Some(cookie_str.to_string());
            }
        }

        let api_response = res.json::<ApiLoginResponse>().await.map_err(|e| format!("解析API响应失败: {}", e))?;
        if api_response.msg != "登录成功" {
            return Ok(LoginResult {
                success: false,
                message: api_response.msg,
            });
        }
        let mut user_storage = USERNAME_STORAGE.lock().unwrap();
        *user_storage = Some(username.clone());
    } else {
        let error_text = res.text().await.unwrap_or_else(|_| "无法读取API错误响应".to_string());
        return Err(format!("API请求返回错误状态: {}", error_text));
    }

    // If API login is successful, proceed with database operations
    let db_url = "postgres://tmstools:521707@honulla.com:5432/tmstools";
    let (mut client, connection) = match tokio_postgres::connect(db_url, NoTls).await {
        Ok(res) => res,
        Err(e) => return Err(format!("数据库连接失败: {}", e)),
    };

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let query = format!("SELECT EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = '{}');", username);
    let row = match client.query_one(&query, &[]).await {
        Ok(row) => row,
        Err(e) => return Err(format!("查询表是否存在时出错: {}", e)),
    };

    let exists: bool = row.get(0);

    if exists {
        Ok(LoginResult {
            success: true,
            message: format!("用户 {} 登录成功", username),
        })
    } else {
        let create_table_query = format!(
            "CREATE TABLE \"{}\" (
                id SERIAL PRIMARY KEY,
                code VARCHAR(255),
                time TIMESTAMP,
                n2n VARCHAR(255),
                q TEXT,
                a TEXT,
                isfeedback INTEGER DEFAULT 0,
                isclose INTEGER DEFAULT 0
            )",
            username
        );
        match client.batch_execute(&create_table_query).await {
            Ok(_) => Ok(LoginResult {
                success: true,
                message: format!("用户 {} 的表已创建", username),
            }),
            Err(e) => Err(format!("创建表时出错: {}", e)),
        }
    }
}

#[tauri::command]
fn get_cookie() -> Option<String> {
    COOKIE_STORAGE.lock().unwrap().clone()
}

#[tauri::command]
async fn get_dashboard_username() -> Result<String, String> {
    let cookie = COOKIE_STORAGE.lock().unwrap().clone();
    if cookie.is_none() {
        return Err("未找到登录Cookie".to_string());
    }
    let cookie = cookie.unwrap();

    let client = reqwest::Client::new();
    let res = client.get("http://c.baobaot.com/admin/dashboard")
        .header("Cookie", cookie)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if res.status().is_success() {
        let body = res.text().await.map_err(|e| format!("读取响应失败: {}", e))?;
        let document = Html::parse_document(&body);
        let selector = Selector::parse("span.username").map_err(|_| "无效的选择器".to_string())?;
        
        if let Some(element) = document.select(&selector).next() {
            let username = element.inner_html();
            Ok(username)
        } else {
            Err("未找到指定的元素".to_string())
        }
    } else {
        Err(format!("请求失败，状态码: {}", res.status()))
    }
}

#[tauri::command]
async fn get_workorders(range: String) -> Result<WorkOrderResponse, String> {
    let username = USERNAME_STORAGE.lock().unwrap().clone().ok_or("用户未登录".to_string())?;
    let db_url = "postgres://tmstools:521707@honulla.com:5432/tmstools";
    let (mut client, connection) = tokio_postgres::connect(db_url, NoTls)
        .await
        .map_err(|e| format!("数据库连接失败: {}", e))?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let query_string = if range == "month" {
        format!(
            "SELECT code, n2n, q, a, isfeedback, isclose FROM \"{}\" WHERE date_trunc('month', time AT TIME ZONE 'Asia/Shanghai') = date_trunc('month', NOW() AT TIME ZONE 'Asia/Shanghai') ORDER BY time DESC",
            username
        )
    } else { // Default to "today"
        format!(
            "SELECT code, n2n, q, a, isfeedback, isclose FROM \"{}\" WHERE time >= date_trunc('day', now() AT TIME ZONE 'Asia/Shanghai') AND time < date_trunc('day', now() AT TIME ZONE 'Asia/Shanghai') + interval '1 day' ORDER BY time DESC",
            username
        )
    };

    let rows = client
        .query(&query_string, &[])
        .await
        .map_err(|e| format!("查询工单失败: {}", e))?;

    let orders: Vec<WorkOrder> = rows
        .into_iter()
        .map(|row| {
            WorkOrder {
                gdid: row.get(0),
                n2n: row.get(1),
                messageq: row.get(2),
                messagea: row.get(3),
                iscreate: 0, // All from DB are considered created
                isfeedback: row.get(4),
                isclose: row.get(5),
            }
        })
        .collect();

    let total = orders.len() as i32;

    Ok(WorkOrderResponse {
        orders,
        total,
    })
}


#[tauri::command]
async fn create_ticket_command(n2p: String, massage_q: String, wx: Option<String>, mobile: Option<String>) -> Result<String, String> {
    let cookie = COOKIE_STORAGE.lock().unwrap().clone().ok_or("未找到登录Cookie")?;
    tms_service::create_ticket(&cookie, &n2p, &massage_q, wx.as_deref(), mobile.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn insert_workorder(code: String, n2n: String, q: String, a: String) -> Result<(), String> {
    let username = USERNAME_STORAGE.lock().unwrap().clone().ok_or("用户未登录".to_string())?;
    let db_url = "postgres://tmstools:521707@honulla.com:5432/tmstools";
    let (mut client, connection) = tokio_postgres::connect(db_url, NoTls)
        .await
        .map_err(|e| format!("数据库连接失败: {}", e))?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let query = format!(
        "INSERT INTO \"{}\" (code, time, n2n, q, a, isfeedback, isclose) VALUES ($1, NOW(), $2, $3, $4, 0, 0)",
        username
    );

    client
        .execute(&query, &[&code, &n2n, &q, &a])
        .await
        .map_err(|e| format!("插入工单失败: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn feedback_workorder(gdid: String) -> Result<(), String> {
    let username = USERNAME_STORAGE.lock().unwrap().clone().ok_or("用户未登录".to_string())?;
    let db_url = "postgres://tmstools:521707@honulla.com:5432/tmstools";
    let (mut client, connection) = tokio_postgres::connect(db_url, NoTls)
        .await
        .map_err(|e| format!("数据库连接失败: {}", e))?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    // 从数据库获取反馈信息
    let row = client.query_one(&format!("SELECT a FROM \"{}\" WHERE code = $1", username), &[&gdid])
        .await
        .map_err(|e| format!("查询反馈信息失败: {}", e))?;
    let messagea: String = row.get(0);

    // 调用远程反馈服务
    let cookie = COOKIE_STORAGE.lock().unwrap().clone().ok_or("未找到登录Cookie")?;
    tms_service::feedback(&cookie, &messagea, &gdid)
        .await
        .map_err(|e| format!("远程反馈失败: {}", e))?;

    // 更新数据库
    let query = format!(
        "UPDATE \"{}\" SET isfeedback = 1 WHERE code = $1",
        username
    );
    client
        .execute(&query, &[&gdid])
        .await
        .map_err(|e| format!("更新数据库失败: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn feedback_today_workorders() -> Result<String, String> {
    let username = USERNAME_STORAGE.lock().unwrap().clone().ok_or("用户未登录".to_string())?;
    let cookie = COOKIE_STORAGE.lock().unwrap().clone().ok_or("未找到登录Cookie")?;
    let db_url = "postgres://tmstools:521707@honulla.com:5432/tmstools";
    let (mut client, connection) = tokio_postgres::connect(db_url, NoTls)
        .await
        .map_err(|e| format!("数据库连接失败: {}", e))?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let query_today_unfeedbacked = format!(
        "SELECT code, a FROM \"{}\" WHERE time >= date_trunc('day', now() AT TIME ZONE 'Asia/Shanghai') AND time < date_trunc('day', now() AT TIME ZONE 'Asia/Shanghai') + interval '1 day' AND isfeedback = 0",
        username
    );

    let rows = client
        .query(&query_today_unfeedbacked, &[])
        .await
        .map_err(|e| format!("查询今日未反馈工单失败: {}", e))?;

    let mut success_count = 0;
    let total_count = rows.len();

    for row in rows {
        sleep(Duration::from_millis(200)).await;
        let gdid: String = row.get(0);
        let messagea: String = row.get(1);

        if !messagea.is_empty() {
            match tms_service::feedback(&cookie, &messagea, &gdid).await {
                Ok(_) => {
                    let update_query = format!(
                        "UPDATE \"{}\" SET isfeedback = 1 WHERE code = $1",
                        username
                    );
                    if client.execute(&update_query, &[&gdid]).await.is_ok() {
                        success_count += 1;
                    }
                }
                Err(_) => {
                    // Log or handle feedback error
                }
            }
        }
    }

    Ok(format!("成功反馈 {}/{} 个工单", success_count, total_count))
}

#[tauri::command]
async fn feedback_selected_workorders(gdids: Vec<String>) -> Result<String, String> {
    let username = USERNAME_STORAGE.lock().unwrap().clone().ok_or("用户未登录".to_string())?;
    let cookie = COOKIE_STORAGE.lock().unwrap().clone().ok_or("未找到登录Cookie")?;
    let db_url = "postgres://tmstools:521707@honulla.com:5432/tmstools";
    let (mut client, connection) = tokio_postgres::connect(db_url, NoTls)
        .await
        .map_err(|e| format!("数据库连接失败: {}", e))?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let mut success_count = 0;
    let total_count = gdids.len();

    for gdid in gdids {
        sleep(Duration::from_millis(200)).await;
        let row = client.query_one(&format!("SELECT a FROM \"{}\" WHERE code = $1", username), &[&gdid])
            .await
            .map_err(|e| format!("查询反馈信息失败: {}", e))?;
        let messagea: String = row.get(0);

        if !messagea.is_empty() {
            match tms_service::feedback(&cookie, &messagea, &gdid).await {
                Ok(_) => {
                    let update_query = format!(
                        "UPDATE \"{}\" SET isfeedback = 1 WHERE code = $1",
                        username
                    );
                    if client.execute(&update_query, &[&gdid]).await.is_ok() {
                        success_count += 1;
                    }
                }
                Err(_) => {
                    // Log or handle feedback error
                }
            }
        }
    }

    Ok(format!("成功反馈 {}/{} 个工单", success_count, total_count))
}

#[tauri::command]
async fn close_workorder(gdid: String) -> Result<(), String> {
    // Step 1: Call the remote close service
    let cookie = COOKIE_STORAGE.lock().unwrap().clone().ok_or("未找到登录Cookie")?;
    tms_service::close(&cookie, &gdid)
        .await
        .map_err(|e| format!("远程关闭失败: {}", e))?;

    // Step 2: If remote close is successful, update the local database
    let username = USERNAME_STORAGE.lock().unwrap().clone().ok_or("用户未登录".to_string())?;
    let db_url = "postgres://tmstools:521707@honulla.com:5432/tmstools";
    let (mut client, connection) = tokio_postgres::connect(db_url, NoTls)
        .await
        .map_err(|e| format!("数据库连接失败: {}", e))?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let query = format!(
        "UPDATE \"{}\" SET isclose = 1 WHERE code = $1",
        username
    );

    client
        .execute(&query, &[&gdid])
        .await
        .map_err(|e| format!("关闭工单失败: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn close_today_workorders() -> Result<String, String> {
    let username = USERNAME_STORAGE.lock().unwrap().clone().ok_or("用户未登录".to_string())?;
    let cookie = COOKIE_STORAGE.lock().unwrap().clone().ok_or("未找到登录Cookie")?;
    let db_url = "postgres://tmstools:521707@honulla.com:5432/tmstools";
    let (mut client, connection) = tokio_postgres::connect(db_url, NoTls)
        .await
        .map_err(|e| format!("数据库连接失败: {}", e))?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let query_today_unclosed = format!(
        "SELECT code FROM \"{}\" WHERE time >= date_trunc('day', now() AT TIME ZONE 'Asia/Shanghai') AND time < date_trunc('day', now() AT TIME ZONE 'Asia/Shanghai') + interval '1 day' AND isclose = 0",
        username
    );

    let rows = client
        .query(&query_today_unclosed, &[])
        .await
        .map_err(|e| format!("查询今日未关闭工单失败: {}", e))?;

    let mut success_count = 0;
    let total_count = rows.len();

    for row in rows {
        sleep(Duration::from_millis(200)).await;
        let gdid: String = row.get(0);
        if tms_service::close(&cookie, &gdid).await.is_ok() {
            let update_query = format!(
                "UPDATE \"{}\" SET isclose = 1 WHERE code = $1",
                username
            );
            if client.execute(&update_query, &[&gdid]).await.is_ok() {
                success_count += 1;
            }
        }
    }

    Ok(format!("成功关闭 {}/{} 个工单", success_count, total_count))
}

#[tauri::command]
async fn close_selected_workorders(gdids: Vec<String>) -> Result<String, String> {
    let username = USERNAME_STORAGE.lock().unwrap().clone().ok_or("用户未登录".to_string())?;
    let cookie = COOKIE_STORAGE.lock().unwrap().clone().ok_or("未找到登录Cookie")?;
    let db_url = "postgres://tmstools:521707@honulla.com:5432/tmstools";
    let (mut client, connection) = tokio_postgres::connect(db_url, NoTls)
        .await
        .map_err(|e| format!("数据库连接失败: {}", e))?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let mut success_count = 0;
    let total_count = gdids.len();

    for gdid in gdids {
        sleep(Duration::from_millis(200)).await;
        if tms_service::close(&cookie, &gdid).await.is_ok() {
            let update_query = format!(
                "UPDATE \"{}\" SET isclose = 1 WHERE code = $1",
                username
            );
            if client.execute(&update_query, &[&gdid]).await.is_ok() {
                success_count += 1;
            }
        }
    }

    Ok(format!("成功关闭 {}/{} 个工单", success_count, total_count))
}

#[tauri::command]
async fn feedback_and_close_today_workorders() -> Result<String, String> {
    let username = USERNAME_STORAGE.lock().unwrap().clone().ok_or("用户未登录".to_string())?;
    let cookie = COOKIE_STORAGE.lock().unwrap().clone().ok_or("未找到登录Cookie")?;
    let db_url = "postgres://tmstools:521707@honulla.com:5432/tmstools";
    let (mut client, connection) = tokio_postgres::connect(db_url, NoTls)
        .await
        .map_err(|e| format!("数据库连接失败: {}", e))?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let query_today_unprocessed = format!(
        "SELECT code, a FROM \"{}\" WHERE time >= date_trunc('day', now() AT TIME ZONE 'Asia/Shanghai') AND time < date_trunc('day', now() AT TIME ZONE 'Asia/Shanghai') + interval '1 day' AND (isfeedback = 0 OR isclose = 0)",
        username
    );

    let rows = client
        .query(&query_today_unprocessed, &[])
        .await
        .map_err(|e| format!("查询今日未处理工单失败: {}", e))?;

    let mut feedback_success_count = 0;
    let mut close_success_count = 0;
    let total_count = rows.len();

    for row in rows {
        sleep(Duration::from_millis(200)).await;
        let gdid: String = row.get(0);
        let messagea: Option<String> = row.get(1);

        // Feedback
        if let Some(msg) = messagea {
            if !msg.is_empty() {
                if tms_service::feedback(&cookie, &msg, &gdid).await.is_ok() {
                    let update_feedback_query = format!(
                        "UPDATE \"{}\" SET isfeedback = 1 WHERE code = $1",
                        username
                    );
                    if client.execute(&update_feedback_query, &[&gdid]).await.is_ok() {
                        feedback_success_count += 1;
                    }
                }
            }
        }

        // Close
        if tms_service::close(&cookie, &gdid).await.is_ok() {
            let update_close_query = format!(
                "UPDATE \"{}\" SET isclose = 1 WHERE code = $1",
                username
            );
            if client.execute(&update_close_query, &[&gdid]).await.is_ok() {
                close_success_count += 1;
            }
        }
    }

    Ok(format!("成功反馈 {}/{} 个, 成功关闭 {}/{} 个", feedback_success_count, total_count, close_success_count, total_count))
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            login,
            get_cookie,
            get_dashboard_username,
            get_workorders,
            create_ticket_command,
            insert_workorder,
            feedback_workorder,
            feedback_today_workorders,
            feedback_selected_workorders,
            close_workorder,
            close_today_workorders,
            close_selected_workorders,
            feedback_and_close_today_workorders
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
