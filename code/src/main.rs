
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

    if opp == "+"{
        num1 += num2;
        println!("This is the answer {}", num1);
    } else if opp == "-" {
        num1 -= num2;
        println!("This is the answer {}", num1);
    } else if opp == "*" {
        num1 *= num2;
        println!("This is the answer {}", num1);
    } else if opp == "/" {
        if num2 == 0 {
            println!("divide by 0 error");
        }
        else {
            num1 /= num2;
            println!("This is the answer {}", num1);
        }
    } else {
        println!("this is not a valid opporator please use +, -, *, or /");
    }   
}
