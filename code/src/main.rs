fn main(){
    let mut task_list : Vec<Task> = Vec::new();
    let mut command = String::new();
    let mut in_use : bool = true;

    use std::slice;

    while in_use == true {
        if task_list.len() > 0 {
            for i in 0..task_list.len(){
                println!("Name: {}", task_list[i].name);
                println!("discription: {}", task_list[i].discription);
                println!("completed: {}", task_list[i].completed);
                println!("------------------------------------");
            }
        } else {
            println!("No tasks added yet");
        }



        println!("------------------------------------");
        println!("enter a command");
        println!("1. add a task");
        println!("2. delete a task");
        println!("3. change a task");
        println!("4. change status");
        std::io::stdin().read_line(&mut command);

        println!("you entered {}", &command);

        match command.trim() {
            "1" => task_list = addtask(task_list),
            "2" => task_list = deletetask(task_list),
            "3" => task_list = changetask(task_list),
            "4" =>{ if task_list.len() == 0 {
                println!("there is nothing to delete");
                } else { 
                    task_list = changestatus(task_list);
                }
            }
            _ => {
                println!("Please enter a number 1-4");
            },
        }
        command.clear();


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
    let mut selected : bool = false;

    println!("What do you want to call the item?");
    std::io::stdin().read_line(&mut name_temp);
   
    println!("What discription do you want it to have?");
    std::io::stdin().read_line(&mut discription_temp);

    while selected == false {

        println!("Is this item completed? yes/no");
        std::io::stdin().read_line(&mut completed_temp);

        if completed_temp.trim() == "yes"{
            completed_bool = true;
            selected = true;
        }
        else if completed_temp.trim() == "no" {
            completed_bool = false;
            selected = true;
        }
        else {
            println!("Please enter yes or no.");
            completed_temp.clear()
        }
    }

    let task_temp = Task {
        name : name_temp,
        discription : discription_temp,
        completed : completed_bool,
    };
    
    println!("You task looks like this:");
    println!("Name: {}", task_temp.name);
    println!("discription: {}", task_temp.discription);
    println!("completed: {}", task_temp.completed);
    println!("------------------------------------");

    list.push(task_temp);
    return list
}
fn deletetask(mut list : Vec<Task>) -> Vec<Task> {
    println!("What task do you want to delete");
    let mut deletion = String::new();
    let mut chosen : bool = false;
    while chosen == false {

        for i in 0..list.len(){

            println!("{}. {}", i + 1, list[i].name);
        }
            std::io::stdin().read_line(&mut deletion);
            let index_deleted : u32 = deletion.parse::<u32>().expect("Please enter a valid number");

            if index_deleted <= 0 {
                println!("Please enter a number from the list");
                deletion.clear();
            }
            else {
                let removed_index : usize = index_deleted as usize - 1 as usize;
                println!("Removed {}", list[removed_index].name);
                list.remove(removed_index);
                chosen = true;
            }
        }
    return list
}
fn changetask(list : Vec<Task>) -> Vec<Task> {
    return list
}
fn changestatus(list : Vec<Task>) -> Vec<Task> {
    return list
}
