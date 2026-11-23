//! `-Ztime-passes` lines for the user crate, not its dependencies.

use crate::error::{ErrorKind, Result};

/// Seconds for `pass` on the compile that follows `Compiling {package}`.
///
/// Dependency passes are printed before that header. The next `Compiling` line
/// ends the user crate's section.
pub(crate) fn user_pass_seconds(log: &str, package: &str, pass: &str) -> Result<f64> {
    let header = format!("Compiling {package} ");
    let mut user = false;
    for line in log.lines() {
        if line.contains(&header) {
            user = true;
            continue;
        }
        if user && line.contains("Compiling ") {
            break;
        }
        if user && pass_name(line) == Some(pass) {
            let Some(seconds) = pass_seconds(line) else {
                return Err(ErrorKind::PassText { pass: pass.to_owned() }.into());
            };
            return Ok(seconds);
        }
    }
    Err(ErrorKind::PassMissing { pass: pass.to_owned(), package: package.to_owned() }.into())
}

fn pass_name(line: &str) -> Option<&str> {
    let name = line.rsplit('\t').next()?.trim();
    if name.is_empty() { None } else { Some(name) }
}

fn pass_seconds(line: &str) -> Option<f64> {
    let rest = line.split_once("time:")?.1;
    let token = rest
        .split(|ch: char| ch.is_whitespace() || ch == ';')
        .find(|token| !token.is_empty())?;
    token.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::user_pass_seconds;

    const LOG: &str = "\
   Compiling ph_beans v0.1.0 (/ws/crates/ph_beans)
time:   0.003; rss:   40MB ->   48MB (   +8MB)\ttype_check_crate
   Compiling bean_map_user v0.1.0 (/ws/target/user)
time:   0.005; rss:   10MB ->   12MB (   +2MB)\texpand_crate
time:   0.067; rss:   50MB ->   80MB (  +30MB)\ttype_check_crate
    Finished dev [unoptimized + debuginfo] target(s) in 0.10s
";

    #[test]
    fn the_user_line_is_the_one_after_its_compiling_header() {
        let seconds = user_pass_seconds(LOG, "bean_map_user", "type_check_crate").unwrap();
        assert!((seconds - 0.067).abs() < 1e-9, "{seconds}");
    }

    #[test]
    fn a_missing_user_pass_is_an_error() {
        let error = user_pass_seconds(LOG, "bean_map_user", "monomorphize_crate")
            .unwrap_err()
            .to_string();
        assert!(error.contains("monomorphize_crate"), "{error}");
        let missing = user_pass_seconds(LOG, "other_user", "type_check_crate").unwrap_err();
        assert!(missing.to_string().contains("other_user"), "{missing}");
    }
}
