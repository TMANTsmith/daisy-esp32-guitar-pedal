fn main() {
    let mut input2 = String::new();
    let mut input1 = String::new();
    let mut opp = String::new();

    println!("number a:");
    let _ = std::io::stdin().read_line(&mut input1);
    let mut x: i32 = input1.trim().parse().expect("failed to parse");

    println!("number b:");
    let _ = std::io::stdin().read_line(&mut input2);
    let mut y: i32 = input2.trim().parse().expect("failed to parse");


    println!("what operation do you want to preform");
    let _ = std::io::stdin().read_line(&mut opp).unwrap();
    let opp = opp.trim(); // <- key line
     
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
        println!("this is not a valid opporator please use +, -, *, or /");
    }   
}
