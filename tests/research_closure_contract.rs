use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_doc(relative: &str) -> String {
    fs::read_to_string(repo_root().join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

fn research_status_row(research_id: &str) -> String {
    read_doc("docs/research-status.md")
        .lines()
        .find(|line| line.starts_with(&format!("| {research_id} |")))
        .unwrap_or_else(|| panic!("missing research-status row for {research_id}"))
        .to_string()
}

#[test]
fn research_status_r001_through_r009_are_accepted() {
    for number in 1..=9 {
        let research_id = format!("R-{number:03}");
        let row = research_status_row(&research_id);
        assert!(
            row.contains("| Accepted |"),
            "{research_id} must be Accepted: {row}"
        );
        assert!(
            row.contains("Resolved:"),
            "{research_id} blocking gap must be resolved: {row}"
        );
        let evidence = row.split('|').nth(3).unwrap_or("").trim();
        assert!(
            !evidence.is_empty(),
            "{research_id} evidence column must not be empty"
        );
    }
}

#[test]
fn owner_decision_tables_have_no_unresolved_rows() {
    let doc = read_doc("docs/research/owner-decisions.md");
    let in_pro_table = doc
        .lines()
        .skip_while(|line| !line.starts_with("| Windows custom clients |"))
        .take_while(|line| line.starts_with('|') && !line.starts_with("|---"))
        .collect::<Vec<_>>();
    assert!(
        !in_pro_table.is_empty(),
        "expected populated Pro Usage Decisions table"
    );
    for row in in_pro_table {
        let cells: Vec<_> = row.split('|').map(str::trim).collect();
        let decision = cells.get(3).copied().unwrap_or("");
        assert!(
            !decision.is_empty() && decision != "Required Decision",
            "Pro usage row must have a recorded decision: {row}"
        );
        assert_ne!(
            decision, "unresolved",
            "Pro usage decision must not be unresolved: {row}"
        );
    }

    let access_rows = doc
        .lines()
        .skip_while(|line| !line.starts_with("| Does OpenDesk need to enforce"))
        .take_while(|line| line.starts_with('|') && !line.starts_with("|---"))
        .collect::<Vec<_>>();
    assert_eq!(
        access_rows.len(),
        4,
        "expected four Access Model decision rows"
    );
    for row in access_rows {
        let cells: Vec<_> = row.split('|').map(str::trim).collect();
        let decision = cells.get(3).copied().unwrap_or("");
        assert!(
            !decision.is_empty() && decision != "Required Decision",
            "access model row must have a recorded decision: {row}"
        );
    }
}

#[test]
fn research_checklists_mark_r001_through_r009_complete() {
    let tasks = read_doc("TASKS.md");
    let checklist = read_doc("docs/feature-checklist.md");
    for number in 1..=9 {
        let token = format!("R-{number:03}");
        let tasks_line = tasks
            .lines()
            .find(|line| line.contains(&token))
            .unwrap_or_else(|| panic!("TASKS.md missing {token}"));
        let checklist_line = checklist
            .lines()
            .find(|line| line.contains(&token))
            .unwrap_or_else(|| panic!("feature-checklist missing {token}"));
        assert!(
            tasks_line.contains("[x]"),
            "TASKS.md must mark {token} complete: {tasks_line}"
        );
        assert!(
            checklist_line.contains("[x]"),
            "feature-checklist must mark {token} complete: {checklist_line}"
        );
    }
}