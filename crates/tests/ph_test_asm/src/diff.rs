//! Unified diff of two normalized probes.

use std::fmt::Write;

const DIFF_LINE_CAP: usize = 2000;
const DIFF_ORIGIN: usize = 1;
const CONTEXT: usize = 3;
const CONTEXT_GAP: usize = CONTEXT * 2;
const HEADER_SLACK: usize = 32;
const MARK_LEN: usize = 1;

struct Hunk {
    left_start: usize,
    right_start: usize,
    lines: Vec<String>,
}

struct Mark {
    left: usize,
    right: usize,
    same: bool,
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
    let mut out = String::with_capacity(
        left_name
            .len()
            .saturating_add(right_name.len())
            .saturating_add(left_text.len())
            .saturating_add(right_text.len())
            .saturating_add(HEADER_SLACK),
    );
    let _ = writeln!(out, "--- {left_name}");
    let _ = writeln!(out, "+++ {right_name}");
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
    if left.len() > DIFF_LINE_CAP || right.len() > DIFF_LINE_CAP {
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
    let mut result = Vec::with_capacity(left.len().saturating_add(right.len()));
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
    let mut result = Vec::with_capacity(left.len().saturating_add(right.len()));
    for line in left {
        result.push(Edit::Delete(line));
    }
    for line in right {
        result.push(Edit::Insert(line));
    }
    result
}

fn prefixed(edit: &Edit<'_>) -> String {
    let (mark, line) = match edit {
        Edit::Same(line) => (" ", *line),
        Edit::Delete(line) => ("-", *line),
        Edit::Insert(line) => ("+", *line),
    };
    let mut text = String::with_capacity(line.len() + MARK_LEN);
    text.push_str(mark);
    text.push_str(line);
    text
}

fn hunks(edits: &[Edit<'_>]) -> Vec<Hunk> {
    let mut marks = Vec::with_capacity(edits.len());
    let mut left_line = DIFF_ORIGIN;
    let mut right_line = DIFF_ORIGIN;
    for edit in edits {
        marks.push(Mark {
            left: left_line,
            right: right_line,
            same: matches!(edit, Edit::Same(_)),
        });
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
    while index < marks.len() {
        if marks[index].same {
            index += 1;
            continue;
        }
        let start = index.saturating_sub(CONTEXT);
        let mut end = index + 1;
        while end < marks.len() {
            let next = marks[end..]
                .iter()
                .position(|item| !item.same)
                .map(|at| end + at);
            let Some(change) = next else {
                break;
            };
            if change - end > CONTEXT_GAP {
                break;
            }
            end = change + 1;
        }
        let stop = (end + CONTEXT).min(marks.len());
        let lines = edits[start..stop].iter().map(prefixed).collect();
        groups.push(Hunk { left_start: marks[start].left, right_start: marks[start].right, lines });
        index = stop;
    }
    groups
}
