fn main() {
    let mut x = String::new();
    let mut y = String::new();
    let mut input = String::new();
    let mut opp = String::new();

    println!("number a:");
    std::io::stdin().read_line(&mut input);
    x = input.trim().parse();

    println!("number b:");
    std::io::stdin().read_line(&mut input);
    y = input.trim().parse();


    println!("what operation do you want to preform");
    std::io::stdin().read_line(&mut opp);

    if opp == "+"{
        x +=y;
        println!("This is the answer {}", x);
    } else if opp == "-" {
        x -= y;
        println!("This is the answer {}", x);
    } else if opp == "*" {
        x *= y;
        println!("This is the answer {}", x);
    } else if opp == "/" {
        if y == 0 {
            println!("divide by 0 error");
        }
        else {
            x /= y;
            println!("This is the answer {}", x);
        }
    } else {
        println!("{} is not a valid opporator please use +, -, *, or /", opp);
    }   
}
