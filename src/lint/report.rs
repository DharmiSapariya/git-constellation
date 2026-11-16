//! The lint report: checks, results and the plain-text rendering.

/// One check and what it found.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    pub name: &'static str,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LintReport {
    pub hard: Vec<Check>,
    pub soft: Vec<Check>,
    /// Checks from Section 10.3 that cannot run until there is drawn output.
    pub deferred: Vec<&'static str>,
}

impl LintReport {
    pub fn hard_passed(&self) -> bool {
        self.hard.iter().all(|c| c.passed)
    }

    pub fn warnings(&self) -> usize {
        self.soft.iter().filter(|c| !c.passed).count()
    }

    /// A plain-text report, one line per check.
    pub fn to_text(&self) -> String {
        let mut out = String::from("HARD checks (must pass)\n");
        for c in &self.hard {
            out.push_str(&format!(
                "  [{}] {}: {}\n",
                if c.passed { "pass" } else { "FAIL" },
                c.name,
                c.detail
            ));
        }
        out.push_str("SOFT checks (warnings)\n");
        for c in &self.soft {
            out.push_str(&format!(
                "  [{}] {}: {}\n",
                if c.passed { " ok " } else { "warn" },
                c.name,
                c.detail
            ));
        }
        out.push_str(&format!(
            "Not checkable yet (need drawn output): {}\n",
            self.deferred.join("; ")
        ));
        out
    }
}
