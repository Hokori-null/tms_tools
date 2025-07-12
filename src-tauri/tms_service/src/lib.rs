use reqwest::cookie::Jar;
use reqwest::header;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
struct ApiResponse {
    msg: String,
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

pub async fn create_ticket(cookie: &str, n2p: &str, massageQ: &str, wx: &str) -> Result<String, reqwest::Error> {
    println!("cookie={},n2n={},问题={}, 微信={}", cookie, n2p, massageQ, wx);

    // 1. Create a cookie jar and a single client for all requests
    let jar = Arc::new(Jar::default());
    let initial_url: reqwest::Url = "http://c.baobaot.com".parse().unwrap();
    // The cookie string can contain multiple cookies separated by ';'.
    // We need to add them one by one.
    for part in cookie.split(';') {
        jar.add_cookie_str(part.trim(), &initial_url);
    }

    let client = reqwest::Client::builder()
        .cookie_provider(jar.clone())
        .build()?;
    println!("开始获取平台id");
    // 2. Get the edit page URL
    let url = format!("http://c.baobaot.com/admin/etms/n2nip?n2nip=&cinemacode={}", n2p);
    let res = client.get(&url).send().await?;
    let body_bytes = res.bytes().await?;
    let body = String::from_utf8_lossy(&body_bytes).to_string();

    println!("源码获取完成");

    let system_prompt = "告诉我编辑按钮的链接，只要告诉我链接的url就可以，不要说其他的任何内容";
    let edit_page_url = openai::ask_openai(&body, system_prompt).await?;
    println!("平台id{}", edit_page_url);

    // 3. Visit the edit page to get its source
    let res2 = client.get(&edit_page_url).send().await?;
    let body2_bytes = res2.bytes().await?;
    let body2 = String::from_utf8_lossy(&body2_bytes).to_string();

    // 4. Get the TMS config URL
    let system_prompt2 = "告诉我TMS配置按钮的链接，只要告诉我链接的url就可以，不要说其他的任何内容";
    let tms_config_url = openai::ask_openai(&body2, system_prompt2).await?;
    println!("tms配置链接{}", tms_config_url);
    
    // 5. Visit the TMS config URL to get the token
    let res3 = client.get(&tms_config_url).send().await?;
    let body3_bytes = res3.bytes().await?;
    let body3 = String::from_utf8_lossy(&body3_bytes).to_string();

    let system_prompt3 = "告诉我权限认证(token)的值，只要告诉我对应的值就行，不要说其他的任何内容";
    let token = openai::ask_openai(&body3, system_prompt3).await?;
    println!("token={}", token);

    // 6. Visit the final URL to get the session cookies
    let final_url = format!("https://c.baobaot.com/cinema/workorder?token={}", token);
    let _final_res = client.get(&final_url).send().await?;

    // 7. Now the jar should contain all necessary cookies.
    // We can proceed to create the work order.
    
    let form_data = &[
        ("type", "99"),
        ("describe", massageQ),
        ("input_file[]", ""),
        ("contact_way", "wx"),
        ("contact", wx),
        ("visit_way", "1"),
    ];

    let post_url = "https://c.baobaot.com/cinema/workorder/ajax_create";

    let res = client.post(post_url).form(form_data).send().await?;
    let response_bytes = res.bytes().await?;
    let response_body = String::from_utf8_lossy(&response_bytes).to_string();
    
    match serde_json::from_str::<ApiResponse>(&response_body) {
        Ok(api_response) => {
            if api_response.msg != "提交成功" {
                return Ok(response_body);
            }
        }
        Err(_) => {
            return Ok(response_body);
        }
    }

    // 8. Get new cookie before fetching work order list
    let final_url = format!("https://c.baobaot.com/cinema/workorder?token={}", token);
    let _final_res = client.get(&final_url).send().await?;

    // 9. Get the work order page to find the latest order
    let workorder_url = "https://c.baobaot.com/cinema/workorder";
    let workorder_res = client.get(workorder_url).send().await?;
    let workorder_bytes = workorder_res.bytes().await?;
    let workorder_html = String::from_utf8_lossy(&workorder_bytes).to_string();

    let system_prompt_details = "找到最新的一个工单所对应的详情按钮的url地址中id的值，只要告诉我对应的值就可以，不要说其他任何内容";
    let details_url = openai::ask_openai(&workorder_html, system_prompt_details).await?;

    Ok(details_url)
}

pub async fn feedback(cookie: &str, messageA: &str, gdID: &str) -> Result<String, reqwest::Error> {
    // 1. Create a cookie jar and a single client for all requests
    let jar = Arc::new(Jar::default());
    let initial_url: reqwest::Url = "https://c.baobaot.com".parse().unwrap();
    // The cookie string can contain multiple cookies separated by ';'.
    // We need to add them one by one.
    for part in cookie.split(';') {
        jar.add_cookie_str(part.trim(), &initial_url);
    }

    let client = reqwest::Client::builder()
        .cookie_provider(jar.clone())
        .build()?;

    // 2. Prepare form data
    let form_data = &[
        ("message", messageA),
        ("input_file[]", ""),
        ("id", gdID),
        ("act", "confirm_feedback"),
    ];

    // 3. Send POST request
    let post_url = "https://c.baobaot.com/admin/workorder/ajax_update";
    let res = client.post(post_url).form(form_data).send().await?;
    let response_bytes = res.bytes().await?;
    let response_body = String::from_utf8_lossy(&response_bytes).to_string();

    Ok(response_body)
}

pub async fn close(cookie: &str, gdID: &str) -> Result<String, reqwest::Error> {
    // 1. Create a cookie jar and a single client for all requests
    let jar = Arc::new(Jar::default());
    let initial_url: reqwest::Url = "https://c.baobaot.com".parse().unwrap();
    // The cookie string can contain multiple cookies separated by ';'.
    // We need to add them one by one.
    for part in cookie.split(';') {
        jar.add_cookie_str(part.trim(), &initial_url);
    }

    let client = reqwest::Client::builder()
        .cookie_provider(jar.clone())
        .build()?;

    // 2. Prepare form data
    let form_data = &[
        ("id", gdID),
        ("act", "close_workorder"),
    ];

    // 3. Send POST request
    let post_url = "https://c.baobaot.com/admin/workorder/ajax_update";
    let res = client.post(post_url).form(form_data).send().await?;
    let response_bytes = res.bytes().await?;
    let response_body = String::from_utf8_lossy(&response_bytes).to_string();

    Ok(response_body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
