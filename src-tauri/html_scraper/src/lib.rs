use scraper::{Html, Selector};
use thiserror::Error;
use regex::Regex;

/// 定义库可能返回的错误类型
#[derive(Debug, Error)]
pub enum ScrapeError {
    /// 当内部使用的CSS选择器无效时返回。
    #[error("无效的CSS选择器: {0}")]
    InvalidSelector(String),
    /// 当正则表达式编译失败时返回。
    #[error("无效的正则表达式: {0}")]
    InvalidRegex(#[from] regex::Error),
}

/// 通用函数：根据CSS选择器和链接文本查找链接。
fn find_link_by_text(html_body: &str, selector_str: &str, link_text: &str) -> Result<Option<String>, ScrapeError> {
    let selector = Selector::parse(selector_str)
        .map_err(|e| ScrapeError::InvalidSelector(format!("'{selector_str}': {e}")))?;

    let document = Html::parse_document(html_body);

    for element in document.select(&selector) {
        if element.text().any(|text| text.trim() == link_text) {
            if let Some(link) = element.value().attr("href") {
                return Ok(Some(link.to_string()));
            }
        }
    }

    Ok(None)
}

/// 解析HTML文本，查找并返回第一个匹配“编辑”按钮的链接。
pub fn find_edit_link(html_body: &str) -> Result<Option<String>, ScrapeError> {
    find_link_by_text(html_body, "a.btn.btn-info.btn-xs.m-bot5", "编辑")
}

/// 解析HTML文本，查找并返回“TMS配置”按钮的链接。
pub fn find_tms_config_link(html_body: &str) -> Result<Option<String>, ScrapeError> {
    find_link_by_text(html_body, "a.btn.btn-default.not-cinema", "TMS配置")
}

/// # 参数
/// * `html_body`: 一个字符串切片，包含要解析的HTML内容。
pub fn find_token(html_body: &str) -> Result<Option<String>, ScrapeError> {
    // 使用属性选择器直接、高效地定位目标元素
    let selector_str = r#"input[name="token"]"#;
    let token_selector = Selector::parse(selector_str)
        .map_err(|e| ScrapeError::InvalidSelector(format!("'{selector_str}': {e}")))?;

    let document = Html::parse_document(html_body);

    // 查找第一个匹配的元素并提取其 "value" 属性
    if let Some(element) = document.select(&token_selector).next() {
        if let Some(token_value) = element.value().attr("value") {
            return Ok(Some(token_value.to_string()));
        }
    }

    Ok(None)
}


/// 解析HTML文本，查找并返回最新一个工单详情按钮链接中的ID。
/// 由于网页将最新的工单放在最上面，我们只需要定位到表格的第一行即可。
pub fn find_latest_work_order_id(html_body: &str) -> Result<Option<String>, ScrapeError> {
    // CSS选择器，用于定位到表格主体(tbody)的第一个表格行(tr)内，class包含"btn-info"的链接(a)
    let selector_str = "tbody tr:first-child a.btn-info";
    let detail_link_selector = Selector::parse(selector_str)
        .map_err(|e| ScrapeError::InvalidSelector(format!("'{selector_str}': {e}")))?;

    let document = Html::parse_document(html_body);

    // 查找第一个匹配的元素
    if let Some(element) = document.select(&detail_link_selector).next() {
        // 提取其 "href" 属性
        if let Some(href) = element.value().attr("href") {
            // 使用正则表达式从URL中提取 "id" 的值
            // 正则表达式 r"id=(\d+)" 匹配 "id=" 后跟着的一串数字(\d+)，并捕获这串数字
            let re = Regex::new(r"id=(\d+)")?;
            if let Some(caps) = re.captures(href) {
                // caps.get(1) 获取第一个捕获组的内容 (也就是\d+匹配到的部分)
                if let Some(id_match) = caps.get(1) {
                    return Ok(Some(id_match.as_str().to_string()));
                }
            }
        }
    }

    Ok(None)
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