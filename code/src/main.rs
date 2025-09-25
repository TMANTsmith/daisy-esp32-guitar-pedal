
fn userInt() -> i32 {
    let mut input1 = String::new();
    
    let _ =  std::io::stdin().read_line(&mut input1);
    let mut x: i32 = input1.trim().parse().expect("failed to parse");
    return x;
}

fn main() {
    let mut opp = String::new();

    println!("number a:");
    let mut num1: i32 = userInt();
    
    println!("number b:");
    let mut num2: i32 = userInt();

    println!("what operation do you want to preform");
    let _ = std::io::stdin().read_line(&mut opp).unwrap();
    let opp = opp.trim(); // <- key line
    
    let result = match opp {
        "+" => Some(num1 + num2),
        "-" => Some(num1 - num2),
        "*" => Some(num1 * num2),
        "/" if num2 != 0 => Some(num1 / num2),
        "/" => {
            println!("Devide by 0 error");
            None
        }
        _ => {
            println!("please enter a valid opp");
            None
        }
    };
    if let Some(answer) = result {
    println!("This is the answer {}", answer); 
    }
}
