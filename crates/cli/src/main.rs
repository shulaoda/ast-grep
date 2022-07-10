use ast_grep_core::AstGrep;
use std::fs::read_to_string;
use std::io::Result;
use clap::Parser;
use std::path::Path;
use ignore::WalkBuilder;
use ansi_term::Style;
use ansi_term::Color::{Cyan, Red, Green};
use similar::{ChangeTag, TextDiff};
use std::fmt::Display;


#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
/**
 * TODO: add some description for ast-grep: sg
 * Example:
 * sg -p ""
 */
struct Args {
    /// AST pattern to match
    #[clap(short,long,value_parser)]
    pattern: String,

    /// String to replace the matched AST node
    #[clap(short, long, value_parser)]
    rewrite: Option<String>,

    /// A comma-delimited list of file extensions to process.
    #[clap(short, long)]
    extensions: Vec<String>,

    /// Include hidden files in search
    #[clap(short,long, parse(from_flag))]
    hidden: bool,

    /// The path whose descendent files are to be explored.
    #[clap(value_parser, default_value=".")]
    path: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let pattern = args.pattern;

    for result in WalkBuilder::new(&args.path).hidden(args.hidden).build() {
        match result {
            Ok(entry) => {
                if let Some(file_type) = entry.file_type() {
                    if !file_type.is_file() {
                        continue;
                    }
                    let path = entry.path();
                    match_one_file(path, &pattern, args.rewrite.as_ref());
                }
            },
            Err(err) => eprintln!("ERROR: {}", err),

        }
    }
    Ok(())
}

fn match_one_file(path: &Path, pattern: &str, rewrite: Option<&String>) {
    let file_content = match read_to_string(&path) {
        Ok(content) => content,
        _ => return,
    };
    let grep = AstGrep::new(file_content);
    let mut matches = grep.root().find_all(pattern).peekable();
    if matches.peek().is_none() {
        return
    }
    println!("{}", Cyan.italic().paint(format!("{}", path.display())));
    if let Some(rewrite) = rewrite {
        for mut e in matches {
            let display = e.display_context();
            let old_str = format!("{}{}{}\n", display.leading, display.matched, display.trailing);
            let new_str = format!("{}{}{}\n", display.leading, e.replace(&pattern, rewrite).unwrap().inserted_text, display.trailing);
            let base_line = display.start_line;
            print_diff(&old_str, &new_str, base_line);
        }
    } else {
        for e in matches {
            let display = e.display_context();
            let leading = Style::new().dimmed().paint(display.leading);
            let trailing = Style::new().dimmed().paint(display.trailing);
            let matched = Style::new().bold().paint(display.matched);
            let highlighted = format!("{leading}{matched}{trailing}");
            let lines: Vec<_> = highlighted.lines().collect();
            let mut num = display.start_line;
            let width = (lines.len() + display.start_line).to_string().chars().count();
            for line in lines {
                let line_num = Style::new().bold().paint(format!("{num}"));
                println!("{line_num:>width$}|{line}");
                num += 1;
            }
        }
    }
}

fn index_display(index: Option<usize>, style: Style) -> impl Display {
    let index_str = match index {
        None => String::from("    "),
        Some(idx) => format!("{:<4}", idx),
    };
    style.paint(index_str)
}

fn print_diff(old: &str, new: &str, base_line: usize) {
    let diff = TextDiff::from_lines(old, new);
    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            println!("{:-^1$}", "-", 80);
        }
        for op in group {
            for change in diff.iter_inline_changes(op) {
                let (sign, s) = match change.tag() {
                    ChangeTag::Delete => ("-", Style::new().fg(Red)),
                    ChangeTag::Insert => ("+", Style::new().fg(Green)),
                    ChangeTag::Equal => (" ", Style::new().dimmed()),
                };
                print!(
                    "{}{}|{}",
                    index_display(change.old_index().map(|i| i + base_line), s),
                    index_display(change.new_index().map(|i| i + base_line), s),
                    s.paint(sign),
                );
                for (emphasized, value) in change.iter_strings_lossy() {
                    if emphasized {
                        print!("{}", s.underline().paint(value));
                    } else {
                        print!("{}", value);
                    }
                }
                if change.missing_newline() {
                    println!();
                }
            }
        }
    }
}
