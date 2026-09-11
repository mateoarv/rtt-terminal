use anyhow::{anyhow, Result};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use probe_rs::config::{Registry, Target};

#[derive(Debug, Clone)]
pub struct TargetMatch {
    pub name: String,
    pub family: String,
    pub score: i64,
}

/// Returns a list of all chip target names available in probe-rs.
pub fn get_all_target_names() -> Result<Vec<(String, String)>> {
    let mut names = Vec::new();
    let registry = Registry::from_builtin_families();

    for family in registry.families() {
        for variant in &family.variants {
            names.push((variant.name.clone(), family.name.clone()));
        }
    }

    names.sort_by(|a, b| a.0.cmp(&b.0));
    names.dedup_by(|a, b| a.0 == b.0);
    Ok(names)
}

/// Performs a fuzzy search on target chip names.
pub fn search_targets(query: &str, limit: usize) -> Result<Vec<TargetMatch>> {
    let all = get_all_target_names()?;
    let matcher = SkimMatcherV2::default();

    let mut matches = Vec::new();
    for (name, family) in all {
        if let Some(score) = matcher.fuzzy_match(&name, query) {
            matches.push(TargetMatch {
                name,
                family,
                score,
            });
        }
    }

    matches.sort_by(|a, b| b.score.cmp(&a.score));
    if matches.len() > limit {
        matches.truncate(limit);
    }
    Ok(matches)
}

/// Validates that a target chip name exists in the database and returns its Target metadata.
pub fn validate_target(name: &str) -> Result<Target> {
    let registry = Registry::from_builtin_families();
    registry.get_target_by_name(name).map_err(|e| anyhow!("Unknown chip '{}': {}", name, e))
}

/// Displays search results to the console.
pub fn print_search_results(query: &str, matches: &[TargetMatch]) {
    if matches.is_empty() {
        println!("No microcontroller chips found matching '{}'.", query);
        return;
    }

    println!("Search results for '{}' (top {} matches):", query, matches.len());
    println!("{:-<60}", "");
    for (i, m) in matches.iter().enumerate() {
        println!("  {:2}. {:<30} (Family: {})", i + 1, m.name, m.family);
    }
    println!("{:-<60}", "");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_target_names() {
        let targets = get_all_target_names().expect("Should load targets");
        assert!(!targets.is_empty(), "Target database should not be empty");
    }

    #[test]
    fn test_search_nrf52() {
        let results = search_targets("nrf52840", 5).expect("Search should succeed");
        assert!(!results.is_empty(), "Should find nRF52840 matches");
        let found = results.iter().any(|m| m.name.to_lowercase().contains("nrf52840"));
        assert!(found, "Should contain nRF52840 in matches");
    }

    #[test]
    fn test_search_stm32f4() {
        let results = search_targets("stm32f401", 5).expect("Search should succeed");
        assert!(!results.is_empty(), "Should find STM32F401 matches");
        let found = results.iter().any(|m| m.name.to_lowercase().contains("stm32f401"));
        assert!(found, "Should contain STM32F401 in matches");
    }

    #[test]
    fn test_validate_known_target() {
        let target = validate_target("nRF52840_xxAA").expect("nRF52840_xxAA should be valid");
        assert_eq!(target.name, "nRF52840_xxAA");
        assert!(!target.cores.is_empty(), "Target should have cores");
    }
}
