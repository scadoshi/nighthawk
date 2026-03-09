const HR_CHAR: &str = "-";
const HR_WIDTH: usize = 50;

/// Prints the startup banner with available commands.
pub fn welcome() {
    println!("Welcome to nighthawk");
    println!("Try the following commands");
    hr();
    command_hint();
    hr();
}

/// Prints a horizontal rule.
pub fn hr() {
    println!("{}", HR_CHAR.repeat(HR_WIDTH));
}

/// Prints the list of available commands with aliases.
pub fn command_hint() {
    println!(
        r"set <key> <value>    (alias: s)
get <key>            (alias: g)
delete <key>         (alias: d, del)
quit                 (alias: q, exit)"
    );
}
