//! Unified diff of two normalized probes.

use std::fmt::Write;

struct Hunk {
    left_start: usize,
    right_start: usize,
    lines: Vec<String>,
}

/// Line diff used when two probes are not equivalent.
pub(crate) fn unified(
    left_name: &str,
    right_name: &str,
    left_text: &str,
    right_text: &str,
) -> String {
    let left: Vec<&str> = left_text.lines().collect();
    let right: Vec<&str> = right_text.lines().collect();
    let edits = edits(&left, &right);
    let mut out = format!("--- {left_name}\n+++ {right_name}\n");
    for hunk in hunks(&edits) {
        let left_count = hunk
            .lines
            .iter()
            .filter(|line| !line.starts_with('+'))
            .count();
        let right_count = hunk
            .lines
            .iter()
            .filter(|line| !line.starts_with('-'))
            .count();
        let _ = writeln!(
            out,
            "@@ -{},{left_count} +{},{right_count} @@",
            hunk.left_start, hunk.right_start
        );
        for line in &hunk.lines {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

enum Edit<'a> {
    Same(&'a str),
    Delete(&'a str),
    Insert(&'a str),
}

fn edits<'a>(left: &'a [&str], right: &'a [&str]) -> Vec<Edit<'a>> {
    if left.len() > 2000 || right.len() > 2000 {
        return fallback(left, right);
    }
    let mut lengths = vec![vec![0usize; right.len() + 1]; left.len() + 1];
    for row in (0..left.len()).rev() {
        for column in (0..right.len()).rev() {
            lengths[row][column] = if left[row] == right[column] {
                lengths[row + 1][column + 1] + 1
            } else {
                lengths[row + 1][column].max(lengths[row][column + 1])
            };
        }
    }
    let mut result = Vec::new();
    let mut row = 0;
    let mut column = 0;
    while row < left.len() && column < right.len() {
        if left[row] == right[column] {
            result.push(Edit::Same(left[row]));
            row += 1;
            column += 1;
        } else if lengths[row + 1][column] >= lengths[row][column + 1] {
            result.push(Edit::Delete(left[row]));
            row += 1;
        } else {
            result.push(Edit::Insert(right[column]));
            column += 1;
        }
    }
    while row < left.len() {
        result.push(Edit::Delete(left[row]));
        row += 1;
    }
    while column < right.len() {
        result.push(Edit::Insert(right[column]));
        column += 1;
    }
    result
}

fn fallback<'a>(left: &'a [&str], right: &'a [&str]) -> Vec<Edit<'a>> {
    let mut result = Vec::new();
    for line in left {
        result.push(Edit::Delete(line));
    }
    for line in right {
        result.push(Edit::Insert(line));
    }
    result
}

fn hunks(edits: &[Edit<'_>]) -> Vec<Hunk> {
    const CONTEXT: usize = 3;
    let mut rendered = Vec::new();
    let mut left_line = 1usize;
    let mut right_line = 1usize;
    for edit in edits {
        let text = match edit {
            Edit::Same(line) => format!(" {line}"),
            Edit::Delete(line) => format!("-{line}"),
            Edit::Insert(line) => format!("+{line}"),
        };
        rendered.push((text, left_line, right_line, matches!(edit, Edit::Same(_))));
        match edit {
            Edit::Same(_) => {
                left_line += 1;
                right_line += 1;
            }
            Edit::Delete(_) => left_line += 1,
            Edit::Insert(_) => right_line += 1,
        }
    }
    let mut groups = Vec::new();
    let mut index = 0;
    while index < rendered.len() {
        if rendered[index].3 {
            index += 1;
            continue;
        }
        let start = index.saturating_sub(CONTEXT);
        let mut end = index + 1;
        while end < rendered.len() {
            let next = rendered[end..]
                .iter()
                .position(|item| !item.3)
                .map(|at| end + at);
            let Some(change) = next else {
                break;
            };
            if change - end > CONTEXT * 2 {
                break;
            }
            end = change + 1;
        }
        let stop = (end + CONTEXT).min(rendered.len());
        let left_start = rendered[start].1;
        let right_start = rendered[start].2;
        let lines = rendered[start..stop]
            .iter()
            .map(|item| item.0.clone())
            .collect();
        groups.push(Hunk {
            left_start,
            right_start,
            lines,
        });
        index = stop;
    }
    groups
}
