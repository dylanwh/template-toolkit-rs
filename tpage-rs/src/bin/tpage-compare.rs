//! `tpage-compare`: run every template in a directory through Perl's
//! `tpage` and `tpage-rs` and report where their output differs.
//!
//! Each template is processed from the directory root (so relative
//! INCLUDEs resolve the same way for both) with a pinned environment:
//! `TZ`, `LC_ALL=C`, and no `~/.tpagerc`.  A case passes when both
//! commands succeed with identical stdout, or both fail.

use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

use clap::Parser;
use color_eyre::eyre::{bail, eyre, Result, WrapErr};
use similar::TextDiff;

/// Perl TT as scripts/fetch-corpora.sh unpacks it.
const BUNDLED_TT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../corpora/Template-Toolkit-3.009"
);

#[derive(Parser, Debug)]
#[command(
    name = "tpage-compare",
    about = "Compare Perl tpage with tpage-rs over a directory of templates"
)]
struct Args {
    /// Directory of templates
    dir: PathBuf,

    /// File extensions to treat as templates (comma separated)
    #[arg(long, value_delimiter = ',', default_value = "tt,tt2")]
    ext: Vec<String>,

    /// Perl tpage command line [default: corpora/Template-Toolkit-3.009]
    #[arg(long, value_name = "CMD")]
    tpage: Option<String>,

    /// tpage-rs binary [default: next to this executable]
    #[arg(long = "tpage-rs", value_name = "PATH")]
    tpage_rs: Option<PathBuf>,

    /// TZ for both commands, so date output is comparable
    #[arg(long, default_value = "UTC")]
    tz: String,

    /// Show stderr for cases where both commands failed
    #[arg(short, long)]
    verbose: bool,

    /// Extra arguments passed to both commands (after --)
    #[arg(last = true)]
    tpage_args: Vec<String>,
}

#[derive(PartialEq)]
enum Verdict {
    Match,
    BothFailed,
    Mismatch,
}

fn perl_tpage(args: &Args) -> Vec<String> {
    match &args.tpage {
        Some(cmd) => cmd.split_whitespace().map(String::from).collect(),
        None => vec![
            "perl".into(),
            format!("-I{}/lib", BUNDLED_TT),
            format!("{}/bin/tpage", BUNDLED_TT),
        ],
    }
}

fn tpage_rs(args: &Args) -> Result<PathBuf> {
    let path = match &args.tpage_rs {
        Some(p) => p.clone(),
        None => std::env::current_exe()?.with_file_name("tpage-rs"),
    };
    if !path.is_file() {
        bail!(
            "{} not found; run `cargo build -p tpage-rs` or pass --tpage-rs",
            path.display()
        );
    }
    Ok(path)
}

fn collect(dir: &Path, exts: &[String], out: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .wrap_err_with(|| format!("reading {}", dir.display()))?
        .collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, exts, out)?;
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| exts.iter().any(|x| x == e))
        {
            out.push(path);
        }
    }
    Ok(())
}

fn run(cmd: &[String], file: &Path, args: &Args) -> Result<Output> {
    let (prog, pre) = cmd.split_first().ok_or_else(|| eyre!("empty command"))?;
    Command::new(prog)
        .args(pre)
        .args(&args.tpage_args)
        .arg(file)
        .current_dir(&args.dir)
        .env("TZ", &args.tz)
        .env("LC_ALL", "C")
        .env("TPAGERC", "/nonexistent/.tpagerc")
        .env_remove("RUST_BACKTRACE")
        .output()
        .wrap_err_with(|| format!("running {}", prog))
}

/// The error message from stderr: color-eyre's "0: ..." line for tpage-rs;
/// for Perl, the first "TYPE error - ..." line (parse errors continue on a
/// second line), else the last line (`die` comes after any warnings).
/// Perl's " at FILE line N." suffix is removed.
fn error_summary(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let text = text.replace("\u{1b}[91m", "").replace("\u{1b}[0m", "");
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let line = lines
        .iter()
        .find_map(|l| l.strip_prefix("0: "))
        .or_else(|| lines.iter().copied().find(|l| l.contains(" error - ")))
        .or_else(|| lines.last().copied())
        .unwrap_or("");
    match line.rfind(" at ") {
        Some(i) if line[i..].contains(" line ") => line[..i].to_string(),
        _ => line.to_string(),
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    let perl = perl_tpage(&args);
    let rust = vec![tpage_rs(&args)?.to_string_lossy().into_owned()];

    let mut files = Vec::new();
    collect(&args.dir, &args.ext, &mut files)?;
    if files.is_empty() {
        bail!(
            "no *.{{{}}} files under {}",
            args.ext.join(","),
            args.dir.display()
        );
    }

    let (mut matched, mut both_failed, mut mismatched) = (0, 0, 0);
    for path in &files {
        let rel = path.strip_prefix(&args.dir).unwrap_or(path);
        let p = run(&perl, rel, &args)?;
        let r = run(&rust, rel, &args)?;
        let (p_out, r_out) = (
            String::from_utf8_lossy(&p.stdout),
            String::from_utf8_lossy(&r.stdout),
        );

        let verdict = match (p.status.success(), r.status.success()) {
            (true, true) if p_out == r_out => Verdict::Match,
            (false, false) => Verdict::BothFailed,
            _ => Verdict::Mismatch,
        };
        match verdict {
            Verdict::Match => {
                matched += 1;
                println!("ok      {}", rel.display());
            }
            Verdict::BothFailed => {
                both_failed += 1;
                println!("error   {} (both failed)", rel.display());
                let (pe, re) = (error_summary(&p.stderr), error_summary(&r.stderr));
                if args.verbose || pe != re {
                    println!("    tpage:    {}\n    tpage-rs: {}", pe, re);
                }
            }
            Verdict::Mismatch => {
                mismatched += 1;
                println!("DIFF    {}", rel.display());
                for (name, out) in [("tpage", &p), ("tpage-rs", &r)] {
                    if !out.status.success() {
                        println!("    {} failed: {}", name, error_summary(&out.stderr));
                    }
                }
                if p.status.success() && r.status.success() {
                    let diff = TextDiff::from_lines(p_out.as_ref(), r_out.as_ref());
                    print!(
                        "{}",
                        diff.unified_diff()
                            .context_radius(2)
                            .header("tpage", "tpage-rs")
                    );
                }
            }
        }
    }

    println!(
        "\n{} files: {} match, {} both failed, {} differ",
        files.len(),
        matched,
        both_failed,
        mismatched
    );
    if mismatched > 0 {
        std::process::exit(1);
    }
    Ok(())
}
