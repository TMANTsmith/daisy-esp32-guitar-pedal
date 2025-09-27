fn main(){
    let mut task_list : Vec<Task> = Vec::new();
    let mut command = String::new();
    let mut in_use : bool = true;

    use std::slice;

    while in_use == true {
        println!("enter a command");
        println!("1. add a task");
        println!("2. delete a task");
        println!("3. change a task");
        println!("4. change status");
        std::io::stdin().read_line(&mut command);

        match command.as_str() {
            "1" => task_list = addtask(task_list),
            "2" => task_list = deletetask(task_list),
            "3" => task_list = changetask(task_list),
            "4" => task_list = changestatus(task_list),
            _ => {
                println!("Please enter a number 1-4");
            },
        }


    }
}
struct Task {
    name : String,
    discription : String,
    completed : bool,
}

fn addtask(mut list : Vec<Task>) -> Vec<Task> {
    let mut name_temp = String::new();
    let mut discription_temp = String::new();
    let mut completed_temp = String::new();
    let mut completed_bool : bool = false;


    println!("What do you want to call the item?");
    std::io::stdin().read_line(&mut name_temp);
   
    println!("What discription do you want it to have?");
    std::io::stdin().read_line(&mut discription_temp);
   
    println!("Is this item completed? yes/no");
    std::io::stdin().read_line(&mut completed_temp);
    if completed_temp == "yes"{
        completed_bool = true;
    }
    else if completed_temp == "no" {
        completed_bool = false;
    }
    else {
        println!("Please enter yes or no.");
    }

    let task_temp = Task {
        name : name_temp,
        discription : discription_temp,
        completed : completed_bool,
    };
    list.push(task_temp);
    return list
}
fn deletetask(list : Vec<Task>) -> Vec<Task> {
    println!("What task do you want to delete");
    let deletion = String::new();

    for i in slice::range(list.len()) {
        i += 1;
        let mut chosen : bool = false;

        while chosen == false; {
            println!("{}. {}" i, list[(i-1)]);
            std::io::stdin().read_line(&mut deletion);
            if deletion.parce() <= 0 {
                println!("Please enter a number from the list");
                deletion.clear();
            }
            else {
                chosen == true;
            }
        }
        i -= 1;
    }
    deletion = ((deletion.parse()) - 1);

    list.remove(deletion);
    return list
}
fn changetask(list : Vec<Task>) -> Vec<Task> {
    return list
}
fn changestatus(list : Vec<Task>) -> Vec<Task> {
    return list
}
