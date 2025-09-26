fn main(){
    let mut task_list = Vec::new();
    let mut command = String::new();

    println!("enter a command");
    println!("1. add a task");
    println!("2. delete a task");
    println!("3. change a task");
    println!("4. change status");
    io::stdin().read_line(&mut, command);
}

struct Task {
    name : String,
    discription : String,
    completed : bool,
}

fn addtask(list : Vec) -> Vec{
    let mut task_name = String::new();
   
    println!("What do you want to call the item?");
    io::stdin().read_line(&mut, task_name);
   
    println!("What discription do you want it to have?");
    io::stdin().read_line(&mut, discription_temp);
   
    println!("Is this item completed?");
    io::stdin().read_line(&mut, completed_temp);
   
    let task_temp = Task {
        name : name_temp,
        discription : discription_temp,
        completed : completed_temp,
    }
    list.push(task_temp);
}

