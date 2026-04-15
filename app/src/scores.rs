use serde::{Deserialize, Serialize};
use std::fs;

const MAX_SCORES: usize = 10;

#[derive(Serialize, Deserialize, Clone)]
pub struct HighScore {
    pub name: String,
    pub score: u32,
}

pub struct ScoreBoard {
    scores: Vec<HighScore>,
    file: &'static str,
}

impl ScoreBoard {
    pub fn load(file: &'static str) -> Self {
        let scores = fs::read_to_string(file)
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<HighScore>>(&s).ok())
            .unwrap_or_default();
        ScoreBoard { scores, file }
    }

    pub fn list(&self) -> &[HighScore] {
        &self.scores
    }

    pub fn min_score(&self) -> u32 {
        if self.scores.len() < MAX_SCORES {
            0
        } else {
            self.scores.last().map(|s| s.score).unwrap_or(0)
        }
    }

    pub fn qualifies(&self, score: u32) -> bool {
        score > 0
            && (self.scores.len() < MAX_SCORES
                || self.scores.last().map_or(true, |s| score > s.score))
    }

    pub fn add(&mut self, name: String, score: u32) -> Option<usize> {
        if !self.qualifies(score) {
            return None;
        }
        let name: String = name.chars().take(20).collect();
        let name = name.trim().to_string();
        if name.is_empty() {
            return None;
        }
        let rank = self.scores.partition_point(|s| s.score > score);
        self.scores.insert(rank, HighScore { name, score });
        self.scores.truncate(MAX_SCORES);
        self.save();
        Some(rank + 1)
    }

    fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.scores) {
            let _ = fs::write(self.file, json);
        }
    }
}
