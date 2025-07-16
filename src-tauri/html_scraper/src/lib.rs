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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
