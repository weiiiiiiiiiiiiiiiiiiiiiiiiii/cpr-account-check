//! 题库、测试快照和可解释的答案判定

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const CANDY_PROMPT: &str = "在一个黑色的袋子里放有三种口味的糖果，每种糖果有两种不同的形状（圆形和五角星形，不同的形状靠手感可以分辨）。现已知不同口味的糖和不同形状的数量统计如下表。参赛者需要在活动前决定摸出的糖果数目，那么，最少取出多少个糖果才能保证手中同时拥有不同形状的苹果味和桃子味的糖？（同时手中有圆形苹果味匹配五角星桃子味糖果，或者有圆形桃子味匹配五角星苹果味糖果都满足要求）\n苹果味 桃子味 西瓜味\n圆形 7 9 8\n五角星形 7 6 4\n直接回答数字答案";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Probe {
    pub id: String,
    pub name: String,
    pub prompt: String,
    pub expected: String,
    pub rule: Rule,
    pub enabled: bool,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    Exact,
    Contains,
    Regex,
    Manual,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub probes: Vec<Probe>,
    pub model: String,
    pub client_key_id: String,
    pub reasoning: String,
    pub timeout_seconds: u64,
    pub repetitions: u8,
    pub concurrency: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            probes: vec![Probe {
                id: "candy".into(),
                name: "糖果题".into(),
                prompt: CANDY_PROMPT.into(),
                expected: "21".into(),
                rule: Rule::Exact,
                enabled: true,
            }],
            model: String::new(),
            client_key_id: String::new(),
            reasoning: "high".into(),
            timeout_seconds: 60,
            repetitions: 1,
            concurrency: 2,
        }
    }
}

pub fn bounded(value: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&value.len()) && !value.contains('\0')
}

impl Probe {
    pub fn validate(&self) -> Result<(), String> {
        if !bounded(&self.id, 1, 128)
            || !bounded(&self.name, 1, 256)
            || !bounded(&self.prompt, 1, 8192)
            || self.prompt.trim().is_empty()
            || !bounded(&self.expected, 0, 2048)
            || (self.rule != Rule::Manual && self.expected.trim().is_empty())
        {
            return Err("题目名称、提示词或预期答案无效".into());
        }
        if self.rule == Rule::Regex {
            compile_regex(&self.expected)?;
        }
        Ok(())
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.probes.is_empty()
            || self.probes.len() > 20
            || !(5..=90).contains(&self.timeout_seconds)
            || !(1..=5).contains(&self.repetitions)
            || !(1..=4).contains(&self.concurrency)
            || ![
                "default", "none", "minimal", "low", "medium", "high", "xhigh",
            ]
            .contains(&self.reasoning.as_str())
            || !bounded(&self.model, 0, 256)
            || !bounded(&self.client_key_id, 0, 256)
        {
            return Err("测试设置无效或超过限制".into());
        }
        if !serde_json::to_vec(self).is_ok_and(|bytes| bytes.len() <= 48 * 1024) {
            return Err("题库与设置总长度不能超过 48 KiB".into());
        }
        let mut ids = BTreeSet::new();
        for probe in &self.probes {
            probe.validate()?;
            if !ids.insert(&probe.id) {
                return Err("题目 ID 重复".into());
            }
        }
        Ok(())
    }
}

fn compile_regex(pattern: &str) -> Result<Regex, String> {
    regex::RegexBuilder::new(pattern)
        .size_limit(1024 * 1024)
        .build()
        .map_err(|_| "正则表达式无效或过于复杂".into())
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Passed,
    WrongAnswer,
    FormatError,
    CallError,
    ManualReview,
}

pub fn judge(probe: &Probe, answer: &str) -> Result<Outcome, String> {
    let answer = answer.trim();
    let expected = probe.expected.trim();
    Ok(match probe.rule {
        Rule::Manual => Outcome::ManualReview,
        Rule::Contains if answer.contains(expected) => Outcome::Passed,
        Rule::Regex if compile_regex(expected)?.is_match(answer) => Outcome::Passed,
        Rule::Exact if answer == expected => Outcome::Passed,
        Rule::Exact if numeric(expected) && !numeric(answer) => Outcome::FormatError,
        _ => Outcome::WrongAnswer,
    })
}

fn numeric(value: &str) -> bool {
    let digits = value.strip_prefix(['+', '-']).unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|c| c.is_ascii_digit())
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunInput {
    pub run_id: String,
    pub batch_id: String,
    pub account_id: String,
    pub probe: Probe,
    pub model: String,
    pub client_key_id: String,
    pub reasoning: String,
    pub timeout_seconds: u64,
    pub repetition: u8,
}

impl RunInput {
    pub fn validate(&self) -> Result<(), String> {
        self.probe.validate()?;
        if !bounded(&self.run_id, 1, 128)
            || !self
                .run_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
        {
            return Err("测试 ID 无效".into());
        }
        if [
            &self.run_id,
            &self.batch_id,
            &self.account_id,
            &self.model,
            &self.client_key_id,
        ]
        .iter()
        .any(|value| !bounded(value, 1, 256))
            || !(5..=90).contains(&self.timeout_seconds)
            || !(1..=5).contains(&self.repetition)
            || ![
                "default", "none", "minimal", "low", "medium", "high", "xhigh",
            ]
            .contains(&self.reasoning.as_str())
        {
            return Err("缺少账号、模型或 Client Key，或测试参数无效".into());
        }
        Ok(())
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResult {
    pub input: RunInput,
    pub account_name: String,
    pub provider: String,
    pub selection_mode: String,
    pub outcome: Outcome,
    pub answer: String,
    pub error_code: Option<String>,
    pub error: Option<String>,
    pub request_id: Option<String>,
    pub response_model: Option<String>,
    pub elapsed_ms: u64,
    pub tested_at_ms: u64,
    pub answer_truncated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn candy_requires_only_the_number() {
        let probe = Settings::default().probes.remove(0);
        for value in ["21", " 21\n", "\t21\r\n"] {
            assert_eq!(judge(&probe, value).unwrap(), Outcome::Passed);
        }
        for value in ["20", "22", "021"] {
            assert_eq!(judge(&probe, value).unwrap(), Outcome::WrongAnswer);
        }
        for value in ["答案是21", "21个", "21\n因为……", "", "21.0"] {
            assert_eq!(judge(&probe, value).unwrap(), Outcome::FormatError);
        }
    }
    #[test]
    fn configurable_rules_and_regex_validation() {
        let mut probe = Settings::default().probes.remove(0);
        probe.rule = Rule::Contains;
        assert_eq!(judge(&probe, "答案是21").unwrap(), Outcome::Passed);
        probe.rule = Rule::Regex;
        probe.expected = "^21$".into();
        assert_eq!(judge(&probe, "21个").unwrap(), Outcome::WrongAnswer);
        probe.expected = "[".into();
        assert!(probe.validate().is_err());
        probe.rule = Rule::Manual;
        probe.expected.clear();
        assert_eq!(judge(&probe, "任何回答").unwrap(), Outcome::ManualReview);
    }
    #[test]
    fn rejects_duplicate_ids_and_unbounded_parameters() {
        let mut settings = Settings::default();
        assert!(settings.validate().is_ok());
        settings.probes.push(settings.probes[0].clone());
        assert!(settings.validate().is_err());
        settings.probes.pop();
        settings.timeout_seconds = 91;
        assert!(settings.validate().is_err());
        settings.timeout_seconds = 60;
        settings.probes = (0..10)
            .map(|index| {
                let mut probe = Settings::default().probes.remove(0);
                probe.id = format!("probe-{index}");
                probe.prompt = "a".repeat(8192);
                probe
            })
            .collect();
        assert!(settings.validate().is_err());
    }
}
