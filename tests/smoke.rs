#[test]
fn lib_modules_are_wired() {
    let _ = gshell::ast::Ast;
    let _ = gshell::builtins::Builtins;
    let _ = gshell::compat::Compat;
    let _ = gshell::completion::Completion;
    let _ = gshell::config::Config;
    let _ = gshell::expand::Expand;
    let _ = gshell::jobs::Jobs;
    let _ = gshell::lexer::Lexer;
    let _ = gshell::parser::Parser;
    let _ = gshell::prompt::Prompt;
    let _ = gshell::runtime::Runtime;
    let _ = gshell::ui::Ui;
}
