// Poterm - Modern TUI editor for .po translation files
// Copyright (c) 2025 AnmiTaliDev <anmitali198@gmail.com>
// Licensed under the Apache License, Version 2.0

use anyhow::{Context, Result};
use chrono;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct PoEntry {
    pub msgid: String,
    pub msgstr: String,
    pub msgctxt: Option<String>,
    pub comments: Vec<String>,
    pub extracted_comments: Vec<String>,
    pub references: Vec<String>,
    pub flags: Vec<String>,
    pub is_fuzzy: bool,
    pub is_translated: bool,
}

impl PoEntry {
    pub fn new() -> Self {
        Self {
            msgid: String::new(),
            msgstr: String::new(),
            msgctxt: None,
            comments: Vec::new(),
            extracted_comments: Vec::new(),
            references: Vec::new(),
            flags: Vec::new(),
            is_fuzzy: false,
            is_translated: false,
        }
    }

    pub fn update_status(&mut self) {
        self.is_fuzzy = self.flags.contains(&"fuzzy".to_string());
        self.is_translated = !self.msgstr.is_empty() && !self.is_fuzzy;
    }

    pub fn set_msgstr(&mut self, msgstr: String) {
        self.msgstr = msgstr;
        self.update_status();
    }

    pub fn toggle_fuzzy(&mut self) {
        if self.is_fuzzy {
            self.flags.retain(|f| f != "fuzzy");
        } else {
            self.flags.push("fuzzy".to_string());
        }
        self.update_status();
    }
}

impl Default for PoEntry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct PoFile {
    pub path: Option<PathBuf>,
    pub header: HashMap<String, String>,
    pub entries: Vec<PoEntry>,
    pub modified: bool,
}

impl PoFile {
    pub fn new(path: PathBuf) -> Self {
        let mut header = HashMap::new();
        header.insert("Project-Id-Version".to_string(), "PACKAGE VERSION".to_string());
        header.insert("Report-Msgid-Bugs-To".to_string(), "".to_string());
        header.insert("POT-Creation-Date".to_string(), "YEAR-MO-DA HO:MI+ZONE".to_string());
        header.insert("PO-Revision-Date".to_string(), "YEAR-MO-DA HO:MI+ZONE".to_string());
        header.insert("Last-Translator".to_string(), "FULL NAME <EMAIL@ADDRESS>".to_string());
        header.insert("Language-Team".to_string(), "LANGUAGE <LL@li.org>".to_string());
        header.insert("Language".to_string(), "".to_string());
        header.insert("MIME-Version".to_string(), "1.0".to_string());
        header.insert("Content-Type".to_string(), "text/plain; charset=UTF-8".to_string());
        header.insert("Content-Transfer-Encoding".to_string(), "8bit".to_string());
        header.insert("Plural-Forms".to_string(), "nplurals=INTEGER; plural=EXPRESSION;".to_string());
        
        Self {
            path: Some(path),
            header,
            entries: Vec::new(),
            modified: false,
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;
        
        let mut po_file = Self::parse(&content)?;
        po_file.path = Some(path.to_path_buf());
        po_file.modified = false;
        
        Ok(po_file)
    }

    pub fn from_pot_template<P: AsRef<Path>>(pot_path: P, po_path: P) -> Result<Self> {
        let pot_path = pot_path.as_ref();
        let po_path = po_path.as_ref();
        
        let content = fs::read_to_string(pot_path)
            .with_context(|| format!("Failed to read POT file: {}", pot_path.display()))?;
        
        let mut po_file = Self::parse(&content)?;
        po_file.path = Some(po_path.to_path_buf());
        
        // Update header for new PO file
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y-%m-%d %H:%M%z").to_string();
        
        po_file.header.insert("PO-Revision-Date".to_string(), timestamp.clone());
        if !po_file.header.contains_key("POT-Creation-Date") || 
           po_file.header.get("POT-Creation-Date").unwrap_or(&String::new()).contains("YEAR-MO-DA") {
            po_file.header.insert("POT-Creation-Date".to_string(), timestamp);
        }
        
        // Clear all msgstr fields for translation
        for entry in &mut po_file.entries {
            if !entry.msgid.is_empty() {  // Don't clear header entry
                entry.msgstr.clear();
                entry.is_translated = false;
                entry.is_fuzzy = false;
                entry.flags.retain(|flag| flag != "fuzzy");
            }
        }
        
        po_file.modified = true;
        Ok(po_file)
    }

    pub fn parse(content: &str) -> Result<Self> {
        let mut po_file = PoFile {
            path: None,
            header: HashMap::new(),
            entries: Vec::new(),
            modified: false,
        };

        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        let mut parse_errors = Vec::new();

        while i < lines.len() {
            let line = lines[i].trim();
            
            // Skip empty lines
            if line.is_empty() {
                i += 1;
                continue;
            }

            // Parse entry
            let mut entry = PoEntry::new();
            let start_i = i;

            // Parse comments and metadata
            while i < lines.len() {
                let line = lines[i].trim();
                if line.is_empty() {
                    break;
                }
                
                if line.starts_with("#.") {
                    entry.extracted_comments.push(line[2..].trim().to_string());
                } else if line.starts_with("#:") {
                    entry.references.push(line[2..].trim().to_string());
                } else if line.starts_with("#,") {
                    let flags: Vec<String> = line[2..]
                        .split(',')
                        .map(|f| f.trim().to_string())
                        .collect();
                    entry.flags.extend(flags);
                } else if line.starts_with('#') && !line.starts_with("#~") {
                    entry.comments.push(line[1..].trim().to_string());
                } else {
                    break;
                }
                i += 1;
            }

            // Parse msgctxt if present
            if i < lines.len() && lines[i].trim().starts_with("msgctxt") {
                entry.msgctxt = Some(Self::parse_string_value(lines[i].trim())?);
                i += 1;
                
                // Handle multiline msgctxt
                while i < lines.len() && lines[i].trim().starts_with('"') {
                    if let Some(ref mut msgctxt) = entry.msgctxt {
                        *msgctxt += &Self::parse_string_literal(lines[i].trim())?;
                    }
                    i += 1;
                }
            }

            // Parse msgid
            if i < lines.len() && lines[i].trim().starts_with("msgid") {
                match Self::parse_string_value(lines[i].trim()) {
                    Ok(msgid) => {
                        entry.msgid = msgid;
                        i += 1;
                        
                        // Handle multiline msgid
                        while i < lines.len() && lines[i].trim().starts_with('"') {
                            match Self::parse_string_literal(lines[i].trim()) {
                                Ok(literal) => entry.msgid += &literal,
                                Err(e) => {
                                    parse_errors.push(format!("Line {}: Failed to parse msgid string literal: {}", i + 1, e));
                                    break;
                                }
                            }
                            i += 1;
                        }
                    }
                    Err(e) => {
                        parse_errors.push(format!("Line {}: Failed to parse msgid: {}", i + 1, e));
                        i += 1;
                    }
                }
            }

            // Parse msgstr
            if i < lines.len() && lines[i].trim().starts_with("msgstr") {
                match Self::parse_string_value(lines[i].trim()) {
                    Ok(msgstr) => {
                        entry.msgstr = msgstr;
                        i += 1;
                        
                        // Handle multiline msgstr
                        while i < lines.len() && lines[i].trim().starts_with('"') {
                            match Self::parse_string_literal(lines[i].trim()) {
                                Ok(literal) => entry.msgstr += &literal,
                                Err(e) => {
                                    parse_errors.push(format!("Line {}: Failed to parse msgstr string literal: {}", i + 1, e));
                                    break;
                                }
                            }
                            i += 1;
                        }
                    }
                    Err(e) => {
                        parse_errors.push(format!("Line {}: Failed to parse msgstr: {}", i + 1, e));
                        i += 1;
                    }
                }
            }

            // Update entry status
            entry.update_status();

            // Handle header entry (msgid is empty)
            if entry.msgid.is_empty() && start_i == 0 {
                // Parse header
                for line in entry.msgstr.lines() {
                    if let Some(colon_pos) = line.find(':') {
                        let key = line[..colon_pos].trim().to_string();
                        let value = line[colon_pos + 1..].trim().to_string();
                        po_file.header.insert(key, value);
                    }
                }
            } else if !entry.msgid.is_empty() {
                po_file.entries.push(entry);
            }
        }

        // Log parse errors if any occurred, but don't fail the entire parse
        if !parse_errors.is_empty() {
            eprintln!("Warning: {} parse errors encountered:", parse_errors.len());
            for error in &parse_errors {
                eprintln!("  {}", error);
            }
        }

        Ok(po_file)
    }

    fn parse_string_value(line: &str) -> Result<String> {
        let re = Regex::new(r#"msg(?:id|str|ctxt)\s+"(.*)""#)?;
        if let Some(captures) = re.captures(line) {
            Self::parse_string_literal(&format!("\"{}\"", &captures[1]))
        } else {
            Ok(String::new())
        }
    }

    fn parse_string_literal(s: &str) -> Result<String> {
        if !s.starts_with('"') || !s.ends_with('"') {
            return Ok(s.to_string());
        }
        
        let content = &s[1..s.len() - 1];
        let mut result = String::new();
        let mut chars = content.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('r') => result.push('\r'),
                    Some('\\') => result.push('\\'),
                    Some('"') => result.push('"'),
                    Some(other) => {
                        result.push('\\');
                        result.push(other);
                    }
                    None => result.push('\\'),
                }
            } else {
                result.push(ch);
            }
        }
        
        Ok(result)
    }

    fn escape_string(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('\n', "\\n")
            .replace('\t', "\\t")
            .replace('\r', "\\r")
            .replace('"', "\\\"")
    }

    pub fn save(&mut self) -> Result<()> {
        if let Some(ref path) = self.path {
            let content = self.to_string();
            fs::write(path, content)
                .with_context(|| format!("Failed to write file: {}", path.display()))?;
            self.modified = false;
        }
        Ok(())
    }

    pub fn save_as<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref().to_path_buf();
        let content = self.to_string();
        fs::write(&path, content)
            .with_context(|| format!("Failed to write file: {}", path.display()))?;
        self.path = Some(path);
        self.modified = false;
        Ok(())
    }

    pub fn to_string(&self) -> String {
        let mut output = String::new();

        // Write header
        if !self.header.is_empty() {
            output.push_str("msgid \"\"\n");
            output.push_str("msgstr \"\"\n");
            for (key, value) in &self.header {
                output.push_str(&format!("\"{}: {}\\n\"\n", key, Self::escape_string(value)));
            }
            output.push('\n');
        }

        // Write entries
        for entry in &self.entries {
            // Write comments
            for comment in &entry.comments {
                output.push_str(&format!("# {}\n", comment));
            }
            
            // Write extracted comments
            for comment in &entry.extracted_comments {
                output.push_str(&format!("#. {}\n", comment));
            }
            
            // Write references
            for reference in &entry.references {
                output.push_str(&format!("#: {}\n", reference));
            }
            
            // Write flags
            if !entry.flags.is_empty() {
                output.push_str(&format!("#, {}\n", entry.flags.join(", ")));
            }

            // Write msgctxt if present
            if let Some(ref msgctxt) = entry.msgctxt {
                output.push_str(&format!("msgctxt \"{}\"\n", Self::escape_string(msgctxt)));
            }

            // Write msgid
            output.push_str(&format!("msgid \"{}\"\n", Self::escape_string(&entry.msgid)));
            
            // Write msgstr
            output.push_str(&format!("msgstr \"{}\"\n", Self::escape_string(&entry.msgstr)));
            
            output.push('\n');
        }

        output
    }

    pub fn mark_modified(&mut self) {
        self.modified = true;
    }

    pub fn get_header(&self) -> &HashMap<String, String> {
        &self.header
    }

    pub fn get_header_mut(&mut self) -> &mut HashMap<String, String> {
        self.modified = true;
        &mut self.header
    }

    pub fn set_header_field(&mut self, key: String, value: String) {
        self.header.insert(key, value);
        self.modified = true;
    }

    pub fn update_revision_date(&mut self) {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y-%m-%d %H:%M%z").to_string();
        self.set_header_field("PO-Revision-Date".to_string(), timestamp);
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn get_stats(&self) -> (usize, usize, usize) {
        let total = self.entries.len();
        let translated = self.entries.iter().filter(|e| e.is_translated).count();
        let fuzzy = self.entries.iter().filter(|e| e.is_fuzzy).count();
        (total, translated, fuzzy)
    }
}

impl Default for PoFile {
    fn default() -> Self {
        Self {
            path: None,
            header: HashMap::new(),
            entries: Vec::new(),
            modified: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_po_entry_new() {
        let entry = PoEntry::new();
        assert_eq!(entry.msgid, "");
        assert_eq!(entry.msgstr, "");
        assert_eq!(entry.msgctxt, None);
        assert!(entry.comments.is_empty());
        assert!(entry.extracted_comments.is_empty());
        assert!(entry.references.is_empty());
        assert!(entry.flags.is_empty());
        assert!(!entry.is_fuzzy);
        assert!(!entry.is_translated);
    }

    #[test]
    fn test_po_entry_update_status() {
        let mut entry = PoEntry::new();
        
        // Test non-fuzzy, translated entry
        entry.msgstr = "Translation".to_string();
        entry.update_status();
        assert!(entry.is_translated);
        assert!(!entry.is_fuzzy);

        // Test fuzzy entry
        entry.flags.push("fuzzy".to_string());
        entry.update_status();
        assert!(!entry.is_translated);
        assert!(entry.is_fuzzy);

        // Test empty msgstr
        entry.flags.clear();
        entry.msgstr.clear();
        entry.update_status();
        assert!(!entry.is_translated);
        assert!(!entry.is_fuzzy);
    }

    #[test]
    fn test_po_entry_set_msgstr() {
        let mut entry = PoEntry::new();
        entry.set_msgstr("Test translation".to_string());
        
        assert_eq!(entry.msgstr, "Test translation");
        assert!(entry.is_translated);
        assert!(!entry.is_fuzzy);
    }

    #[test]
    fn test_po_entry_toggle_fuzzy() {
        let mut entry = PoEntry::new();
        
        // Toggle from non-fuzzy to fuzzy
        entry.toggle_fuzzy();
        assert!(entry.flags.contains(&"fuzzy".to_string()));
        assert!(entry.is_fuzzy);
        
        // Toggle back from fuzzy to non-fuzzy
        entry.toggle_fuzzy();
        assert!(!entry.flags.contains(&"fuzzy".to_string()));
        assert!(!entry.is_fuzzy);
    }

    #[test]
    fn test_escape_unescape_string() {
        // Test escaping
        assert_eq!(PoFile::escape_string("test\\nstring"), "test\\\\nstring");
        assert_eq!(PoFile::escape_string("test\"quote"), "test\\\"quote");
        assert_eq!(PoFile::escape_string("test\nline"), "test\\nline");
        assert_eq!(PoFile::escape_string("test\ttab"), "test\\ttab");

        // Test unescaping through parse_string_literal
        assert_eq!(PoFile::parse_string_literal("\"test\\\\nstring\"").unwrap(), "test\\nstring");
        assert_eq!(PoFile::parse_string_literal("\"test\\\"quote\"").unwrap(), "test\"quote");
        assert_eq!(PoFile::parse_string_literal("\"test\\nline\"").unwrap(), "test\nline");
        assert_eq!(PoFile::parse_string_literal("\"test\\ttab\"").unwrap(), "test\ttab");
    }

    #[test]
    fn test_po_file_new() {
        use std::path::PathBuf;
        let path = PathBuf::from("test.po");
        let po_file = PoFile::new(path.clone());
        
        assert_eq!(po_file.path, Some(path));
        assert!(!po_file.modified);
        assert!(po_file.entries.is_empty()); // New file starts with empty entries
        assert!(!po_file.header.is_empty()); // Should have default headers
    }

    #[test]
    fn test_po_file_stats() {
        let mut po_file = PoFile::default();
        
        // Add test entries
        let mut entry1 = PoEntry::new();
        entry1.msgid = "Test 1".to_string();
        entry1.set_msgstr("Translation 1".to_string());
        po_file.entries.push(entry1);

        let mut entry2 = PoEntry::new();
        entry2.msgid = "Test 2".to_string();
        entry2.flags.push("fuzzy".to_string());
        entry2.update_status();
        po_file.entries.push(entry2);

        let mut entry3 = PoEntry::new();
        entry3.msgid = "Test 3".to_string();
        po_file.entries.push(entry3);

        let (total, translated, fuzzy) = po_file.get_stats();
        let untranslated = total - translated - fuzzy;
        assert_eq!(total, 3);
        assert_eq!(translated, 1);
        assert_eq!(fuzzy, 1);
        assert_eq!(untranslated, 1);
    }

    #[test]
    fn test_from_pot_template() {
        // Create a mock POT content
        let pot_content = r#"# SOME DESCRIPTIVE TITLE.
# Copyright (C) YEAR THE PACKAGE'S COPYRIGHT HOLDER
#
msgid ""
msgstr ""
"Project-Id-Version: PACKAGE VERSION\n"
"Report-Msgid-Bugs-To: \n"
"POT-Creation-Date: 2023-01-01 12:00+0000\n"
"PO-Revision-Date: YEAR-MO-DA HO:MI+ZONE\n"
"Last-Translator: FULL NAME <EMAIL@ADDRESS>\n"
"Language-Team: LANGUAGE <LL@li.org>\n"
"Language: \n"
"MIME-Version: 1.0\n"
"Content-Type: text/plain; charset=UTF-8\n"
"Content-Transfer-Encoding: 8bit\n"

msgid "Hello World"
msgstr ""

msgid "Goodbye"
msgstr ""
"#;
        
        // Write to temp POT file
        use std::io::Write;
        let mut pot_file = tempfile::NamedTempFile::new().unwrap();
        pot_file.write_all(pot_content.as_bytes()).unwrap();
        
        // Create PO from POT
        use std::path::PathBuf;
        let po_path = PathBuf::from("/tmp/test.po");
        let po_file = PoFile::from_pot_template(pot_file.path(), &po_path).unwrap();
        
        // Check that msgstr fields are cleared
        assert_eq!(po_file.entries.len(), 2);
        for entry in &po_file.entries {
            if !entry.msgid.is_empty() {  // Skip header entry
                assert!(entry.msgstr.is_empty());
                assert!(!entry.is_translated);
                assert!(!entry.is_fuzzy);
            }
        }
        
        // Check that PO-Revision-Date is updated
        assert!(po_file.header.get("PO-Revision-Date").unwrap() != "YEAR-MO-DA HO:MI+ZONE");
        assert!(po_file.modified);
    }

    #[test]
    fn test_metadata_functions() {
        let mut po_file = PoFile::default();
        
        // Test setting header field
        po_file.set_header_field("Language".to_string(), "ru".to_string());
        assert_eq!(po_file.get_header().get("Language").unwrap(), "ru");
        assert!(po_file.is_modified());
        
        // Test updating revision date
        po_file.update_revision_date();
        let revision_date = po_file.get_header().get("PO-Revision-Date").unwrap();
        assert!(!revision_date.contains("YEAR-MO-DA"));
    }
}