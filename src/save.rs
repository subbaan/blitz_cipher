use std::fs;
use std::path::PathBuf;

use crate::keybinds::{self, KeyBindings};

#[derive(Clone)]
pub struct HighScoreEntry {
    pub name: String,
    pub score: u32,
    pub stage: usize,
}

pub struct SaveData {
    pub high_scores: Vec<HighScoreEntry>,
    pub keybindings: KeyBindings,
}

impl SaveData {
    fn save_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        Some(PathBuf::from(home).join(".config").join("blitz_cipher").join("save.json"))
    }

    pub fn default() -> Self {
        Self {
            high_scores: Vec::new(),
            keybindings: KeyBindings::default(),
        }
    }

    pub fn load() -> Self {
        let path = match Self::save_path() {
            Some(p) => p,
            None => return Self::default(),
        };
        let contents = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        Self::parse(&contents).unwrap_or_else(|| Self::default())
    }

    pub fn save(&self) {
        let path = match Self::save_path() {
            Some(p) => p,
            None => return,
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = self.to_json();
        let _ = fs::write(&path, json);
    }

    /// Returns the rank index (0-based) if the score qualifies for top 10
    pub fn qualifies(&self, score: u32) -> Option<usize> {
        if score == 0 {
            return None;
        }
        for (i, entry) in self.high_scores.iter().enumerate() {
            if score > entry.score {
                return Some(i);
            }
        }
        if self.high_scores.len() < 10 {
            Some(self.high_scores.len())
        } else {
            None
        }
    }

    /// Insert a new score at the given rank and truncate to 10
    pub fn insert(&mut self, rank: usize, name: String, score: u32, stage: usize) {
        self.high_scores.insert(rank, HighScoreEntry { name, score, stage });
        self.high_scores.truncate(10);
    }

    pub fn top_score(&self) -> u32 {
        self.high_scores.first().map(|e| e.score).unwrap_or(0)
    }

    fn to_json(&self) -> String {
        let mut s = String::from("{\n  \"high_scores\": [\n");
        for (i, entry) in self.high_scores.iter().enumerate() {
            s.push_str(&format!(
                "    {{ \"name\": \"{}\", \"score\": {}, \"stage\": {} }}",
                escape_json(&entry.name), entry.score, entry.stage
            ));
            if i + 1 < self.high_scores.len() {
                s.push(',');
            }
            s.push('\n');
        }
        s.push_str("  ],\n  \"keybindings\": [\n");
        for i in 0..8 {
            let key_str = keybinds::keycode_to_string(self.keybindings.get(i));
            s.push_str(&format!("    \"{}\"", escape_json(&key_str)));
            if i < 7 {
                s.push(',');
            }
            s.push('\n');
        }
        s.push_str("  ]\n}\n");
        s
    }

    fn parse(contents: &str) -> Option<Self> {
        let mut data = Self::default();

        // Parse high_scores array
        if let Some(scores_start) = contents.find("\"high_scores\"") {
            if let Some(arr_start) = contents[scores_start..].find('[') {
                let arr_start = scores_start + arr_start;
                if let Some(arr_end) = contents[arr_start..].find(']') {
                    let arr_end = arr_start + arr_end;
                    let arr_content = &contents[arr_start + 1..arr_end];

                    // Parse each { ... } entry
                    let mut pos = 0;
                    let bytes = arr_content.as_bytes();
                    while pos < bytes.len() {
                        if let Some(obj_start) = arr_content[pos..].find('{') {
                            let obj_start = pos + obj_start;
                            if let Some(obj_end) = arr_content[obj_start..].find('}') {
                                let obj_end = obj_start + obj_end;
                                let obj = &arr_content[obj_start + 1..obj_end];
                                if let Some(entry) = Self::parse_score_entry(obj) {
                                    data.high_scores.push(entry);
                                }
                                pos = obj_end + 1;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        // Parse keybindings array
        if let Some(kb_start) = contents.find("\"keybindings\"") {
            if let Some(arr_start) = contents[kb_start..].find('[') {
                let arr_start = kb_start + arr_start;
                if let Some(arr_end) = contents[arr_start..].find(']') {
                    let arr_end = arr_start + arr_end;
                    let arr_content = &contents[arr_start + 1..arr_end];

                    let mut keys: Vec<String> = Vec::new();
                    let mut pos = 0;
                    while pos < arr_content.len() {
                        if let Some(q1) = arr_content[pos..].find('"') {
                            let q1 = pos + q1 + 1;
                            if let Some(q2) = arr_content[q1..].find('"') {
                                let q2 = q1 + q2;
                                keys.push(arr_content[q1..q2].to_string());
                                pos = q2 + 1;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    for (i, key_str) in keys.iter().enumerate().take(8) {
                        if let Some(kc) = keybinds::string_to_keycode(key_str) {
                            data.keybindings.set(i, kc);
                        }
                    }
                }
            }
        }

        data.high_scores.truncate(10);
        Some(data)
    }

    fn parse_score_entry(obj: &str) -> Option<HighScoreEntry> {
        let name = Self::extract_string_field(obj, "name")?;
        let score = Self::extract_number_field(obj, "score")? as u32;
        let stage = Self::extract_number_field(obj, "stage")? as usize;
        Some(HighScoreEntry {
            name: name.chars().take(3).collect(),
            score,
            stage,
        })
    }

    fn extract_string_field(obj: &str, field: &str) -> Option<String> {
        let key = format!("\"{}\"", field);
        let pos = obj.find(&key)?;
        let after_key = pos + key.len();
        let colon = obj[after_key..].find(':')?;
        let after_colon = after_key + colon + 1;
        let q1 = obj[after_colon..].find('"')? + after_colon + 1;
        let q2 = obj[q1..].find('"')? + q1;
        Some(obj[q1..q2].to_string())
    }

    fn extract_number_field(obj: &str, field: &str) -> Option<i64> {
        let key = format!("\"{}\"", field);
        let pos = obj.find(&key)?;
        let after_key = pos + key.len();
        let colon = obj[after_key..].find(':')?;
        let after_colon = after_key + colon + 1;
        let rest = obj[after_colon..].trim_start();
        let num_end = rest.find(|c: char| !c.is_ascii_digit() && c != '-').unwrap_or(rest.len());
        rest[..num_end].parse().ok()
    }
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
