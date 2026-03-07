const HR_CHAR: &str = "-";
const HR_WIDTH: usize = 50;

pub fn welcome() {
    println!("Welcome to nighthawk");
    println!("Try the following commands");
    hr();
    command_hint();
    hr();
}

pub fn hr() {
    println!("{}", HR_CHAR.repeat(HR_WIDTH));
}

pub fn command_hint() {
    println!(
        r"set <key> <value>    (alias: s)
get <key>            (alias: g)
delete <key>         (alias: d, del)
quit                 (alias: q, exit)"
    );
}
