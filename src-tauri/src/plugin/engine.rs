use super::types::{PluginConfigField, PluginInfo, PluginSource};
use std::path::Path;

/// Plugin code validator and metadata extractor.
/// Parses plugin JS code to extract metadata without executing it.
#[derive(Clone)]
pub struct PluginEngine {
    info: PluginInfo,
    sources: Vec<PluginSource>,
    plugin_type: String,
    config_schema: Vec<PluginConfigField>,
    code: String,
}

impl PluginEngine {
    /// Create a new engine by parsing plugin code metadata.
    pub fn new(plugin_code: &str) -> Result<Self, String> {
        let info = Self::extract_plugin_info(plugin_code)?;
        let sources = Self::extract_sources(plugin_code);
        let plugin_type = Self::extract_plugin_type(plugin_code);
        let config_schema = if plugin_type == "service" {
            Self::extract_config_schema(plugin_code)
        } else {
            Vec::new()
        };

        Ok(Self {
            info,
            sources,
            plugin_type,
            config_schema,
            code: plugin_code.to_string(),
        })
    }

    /// Create engine from file path.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let code = std::fs::read_to_string(path).map_err(|e| format!("读取插件文件失败: {}", e))?;
        Self::new(&code)
    }

    pub fn info(&self) -> &PluginInfo {
        &self.info
    }
    pub fn sources(&self) -> &[PluginSource] {
        &self.sources
    }
    pub fn plugin_type(&self) -> &str {
        &self.plugin_type
    }
    pub fn config_schema(&self) -> &[PluginConfigField] {
        &self.config_schema
    }
    #[allow(dead_code)]
    pub fn code(&self) -> &str {
        &self.code
    }

    fn extract_plugin_type(code: &str) -> String {
        if let Some(val) = Self::extract_string_field(code, "pluginType") {
            return val;
        }
        // Heuristic: service plugins typically have configSchema or getPlaylists
        if code.contains("configSchema") || code.contains("getPlaylists") {
            return "service".to_string();
        }
        // Heuristic: music source plugins have source/quality definitions
        if code.contains("music-search") || code.contains("getMusicUrl") {
            return "music-source".to_string();
        }
        "music-source".to_string()
    }

    /// Extract configSchema array from service plugin JS code.
    fn extract_config_schema(code: &str) -> Vec<PluginConfigField> {
        let mut fields = Vec::new();

        // Find configSchema = [ ... ] block
        let schema_start = match code.find("configSchema") {
            Some(pos) => {
                let rest = &code[pos + "configSchema".len()..];
                let rest = rest.trim_start();
                if rest.starts_with('=') || rest.starts_with(':') {
                    let after_eq = rest[1..].trim_start();
                    match after_eq.find('[') {
                        Some(i) => pos + "configSchema".len() + 1 + i + 1,
                        None => return fields,
                    }
                } else {
                    return fields;
                }
            }
            None => return fields,
        };

        // Find matching closing bracket
        let schema_code = &code[schema_start.saturating_sub(1)..];
        let end = match Self::find_matching_bracket(schema_code, '[', ']') {
            Some(e) => e,
            None => return fields,
        };

        let inner = &schema_code[1..end];

        // Split by top-level objects (delimiter: }, {)
        let obj_strings = Self::split_top_level_objects(inner);

        for obj_str in obj_strings {
            if let Some(field) = Self::parse_config_field(obj_str) {
                fields.push(field);
            }
        }

        fields
    }

    fn find_matching_bracket(code: &str, open: char, close: char) -> Option<usize> {
        let mut depth = 0usize;
        let mut quote = None;
        let mut escaped = false;
        let start = code.find(open)?;
        for (i, c) in code[start..].char_indices() {
            if let Some(quote_char) = quote {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == quote_char {
                    quote = None;
                }
                continue;
            }

            match c {
                '"' | '\'' | '`' => quote = Some(c),
                _ if c == open => depth += 1,
                _ if c == close => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(start + i);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn split_top_level_objects(inner: &str) -> Vec<&str> {
        let mut result = Vec::new();
        let mut depth = 0;
        let mut start = None;

        for (i, c) in inner.char_indices() {
            match c {
                '{' => {
                    if depth == 0 {
                        start = Some(i);
                    }
                    depth += 1;
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(s) = start {
                            result.push(&inner[s..=i]);
                        }
                    }
                }
                _ => {}
            }
        }
        result
    }

    fn parse_config_field(obj_str: &str) -> Option<PluginConfigField> {
        let key = Self::extract_string_field(obj_str, "key")?;
        let label = Self::extract_string_field(obj_str, "label").unwrap_or_else(|| key.clone());
        let field_type =
            Self::extract_string_field(obj_str, "type").unwrap_or_else(|| "text".to_string());
        let required = Self::extract_bool_field(obj_str, "required");
        let placeholder = Self::extract_string_field(obj_str, "placeholder");
        let default =
            Self::extract_string_field(obj_str, "default").map(|v| serde_json::Value::String(v));

        Some(PluginConfigField {
            key,
            label,
            field_type,
            required,
            default,
            placeholder,
            options: None,
        })
    }

    fn extract_bool_field(code: &str, field: &str) -> bool {
        let patterns = [
            format!(r"{}\s*:\s*true", field),
            format!(r#""{}"\s*:\s*true"#, field),
        ];
        for p in &patterns {
            if regex_lite::Regex::new(p)
                .map(|re| re.is_match(code))
                .unwrap_or(false)
            {
                return true;
            }
        }
        false
    }

    fn extract_string_field(code: &str, field: &str) -> Option<String> {
        Self::extract_object_field(code, field).or_else(|| Self::extract_comment_field(code, field))
    }

    /// Extract a field value from @field comment format (e.g. @name PluginName).
    fn extract_comment_field(code: &str, field: &str) -> Option<String> {
        let pattern = format!(r"@{}\s+(.+)", field);
        regex_lite::Regex::new(&pattern)
            .ok()?
            .captures(code)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
    }

    /// Extract pluginInfo from JS code by parsing the object literal.
    fn extract_plugin_info(code: &str) -> Result<PluginInfo, String> {
        let name =
            Self::extract_string_field(code, "name").unwrap_or_else(|| "未知插件".to_string());
        let version =
            Self::extract_string_field(code, "version").unwrap_or_else(|| "1.0.0".to_string());
        let author =
            Self::extract_string_field(code, "author").unwrap_or_else(|| "未知作者".to_string());
        let description = Self::extract_string_field(code, "description").unwrap_or_default();

        if name == "未知插件" {
            return Err("无法解析插件名称".to_string());
        }

        Ok(PluginInfo {
            name,
            version,
            author,
            description,
        })
    }

    /// Extract a string field value from JS code.
    fn extract_object_field(code: &str, field: &str) -> Option<String> {
        let patterns = [
            format!(r#"{}\s*:\s*"([^"]*)""#, field),
            format!(r"{}\s*:\s*'([^']*)'", field),
            format!(r#"{}\s*:\s*`([^`]*)`"#, field),
        ];

        for pattern in &patterns {
            if let Some(caps) = regex_lite::Regex::new(pattern).ok()?.captures(code) {
                if let Some(m) = caps.get(1) {
                    return Some(m.as_str().to_string());
                }
            }
        }
        None
    }

    /// Extract source definitions from JS code.
    fn extract_sources(code: &str) -> Vec<PluginSource> {
        let mut sources = Vec::new();

        let source_ids = ["kw", "kg", "tx", "wy", "mg", "mgt", "bd", "local"];
        let source_names = [
            "小蜗",
            "小苟",
            "小鹅",
            "小芸",
            "菇菇",
            "菇菇",
            "百度音乐",
            "本地",
        ];

        for (id, default_name) in source_ids.iter().zip(source_names.iter()) {
            if code.contains(&format!("\"{}\"", id)) || code.contains(&format!("'{}'", id)) {
                let qualities = Self::extract_qualities_for_source(code, id);
                sources.push(PluginSource {
                    source_id: id.to_string(),
                    name: default_name.to_string(),
                    qualities,
                });
            }
        }

        sources
    }

    /// Extract quality list for a specific source.
    fn extract_qualities_for_source(code: &str, source_id: &str) -> Vec<String> {
        let default_qualities = vec![
            "128k".to_string(),
            "320k".to_string(),
            "flac".to_string(),
            "flac24bit".to_string(),
            "hires".to_string(),
        ];

        let sid = regex_lite::escape(source_id);

        let patterns = [
            // Converted LX format: source_id: { ..., qualitys: [...] }
            format!(r#"{sid}\s*:\s*\{{[^}}]*?qualitys\s*:\s*\[([^\]]*)\]"#),
            // Quoted key: 'source_id': { ..., qualitys: [...] }
            format!(r#"'{sid}'\s*:\s*\{{[^}}]*?qualitys\s*:\s*\[([^\]]*)\]"#),
            // Double-quoted key: "source_id": { ..., qualitys: [...] }
            format!(r#""{sid}"\s*:\s*\{{[^}}]*?qualitys\s*:\s*\[([^\]]*)\]"#),
            // Direct array: 'source_id': [...]
            format!(r#"'{sid}'\s*:\s*\[([^\]]*)\]"#),
            // Direct array: "source_id": [...]
            format!(r#""{sid}"\s*:\s*\[([^\]]*)\]"#),
        ];

        for pattern in &patterns {
            if let Ok(re) = regex_lite::Regex::new(pattern) {
                if let Some(caps) = re.captures(code) {
                    if let Some(m) = caps.get(1) {
                        let quals: Vec<String> = m
                            .as_str()
                            .split(',')
                            .filter_map(|q| {
                                let q = q.trim().trim_matches(|c| c == '\'' || c == '"');
                                if q.is_empty() {
                                    None
                                } else {
                                    Some(q.to_string())
                                }
                            })
                            .collect();
                        if !quals.is_empty() {
                            return quals;
                        }
                    }
                }
            }
        }

        default_qualities
    }
}

#[cfg(test)]
mod tests {
    use super::PluginEngine;

    #[test]
    fn escaped_non_ascii_does_not_panic() {
        let code = r#"const configSchema = ["\你"];"#;

        assert!(std::panic::catch_unwind(|| { PluginEngine::extract_config_schema(code) }).is_ok());
    }
}
