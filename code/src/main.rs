fn main() {
    let mut x = 0;
    let mut y = 0;
    let mut opp = String::new();

    println!("number a:");
    std::io::stdin().read_line(&mut x);
    
    println!("number b:");
    std::io::stdin().read_line(&mut y);
    
    println!("what operation do you want to preform");
    std::io::stdin().read_line(&mut opp);

    if opp == "+"{
        x +=y;
        println!("This is the answer {x}");
    } else if opp == "-" {
        x -= y;
        println!("This is the answer {x}");
    } else if opp == "*" {
        x *= y;
        println!("This is the answer {x}");
    } else if opp == "/" {
        if y == 0 {
            println!("divide by 0 error");
        }
        else {
            x /= y;
            println!("This is the answer {x}");
        }
    } else {
        println!("{opp} is not a valid opporator please use +, -, *, or /");
    }   
}
