#![no_std]
use soroban_sdk::{
    contract, contractimpl, Address, Env, Symbol, Vec, Map, symbol_short
};

// Imports
mod storage;
mod events;
mod storage_types;

// Function from external files
use storage::{get_address, get_project, get_all_projects};

// Get struct
use crate::storage_types::{Objective, Project};

#[contract]
pub struct FreelanceContract;

#[contractimpl]
impl FreelanceContract {

    pub fn create_project(e: Env, freelancer: Address, prices: Vec<u128>) -> u128 {
        let contract_key = symbol_short!("p_count");

        let mut project_count: u128 = e
            .storage()
            .instance()
            .get(&contract_key)
            .unwrap_or(0);

        project_count += 1;
        e.storage().instance().set(&contract_key, &project_count);

        let mut objectives: Map<u128, Objective> = Map::new(&e);
        for (i, price) in prices.iter().enumerate() {
            objectives.set(i as u128, Objective {
                price: price,
                half_paid: 0,
                completed: false,
            });
        }

        let project = Project {
            client: get_address(&e, 0),
            freelancer: freelancer.clone(),
            objectives_count: prices.len() as u128,
            objectives,
            completed_objectives: 0,
            earned_amount: 0,
            contract_balance: 0,
            cancelled: false,
            completed: false,
        };

        let mut key_bytes = [0u8; 32];
        let prefix_bytes = b"project_";
        key_bytes[..8].copy_from_slice(prefix_bytes);
        key_bytes[8..16].copy_from_slice(&project_count.to_le_bytes());

        let key_str = core::str::from_utf8(&key_bytes).unwrap();

        let project_key = Symbol::new(&e, key_str);
        e.storage().instance().set(&project_key, &project);

        // Emitir el evento con el projecto
        events::project_created(&e, project_key.clone(), get_address(&e, 0), freelancer, prices);

        project_count
    }
    
    pub fn complete_project(e: Env, project_id: u128) {

        // Obtener el proyecto
        let (mut project, project_key) = get_project(&e, project_id);

        // Verificar que la persona que invoca la función es el cliente
        let invoker = get_address(&e, 0);
        if invoker != project.client {
            panic!("Only the client can mark the project as completed");
        }

        // Check if the project is cancelled
        if !project.completed {
            panic!("Project is completed");
        }

        // Check if the project is cancelled
        if !project.cancelled {
            panic!("Project is cancelled");
        }

        // Check if all the objectives are completed
        if project.completed_objectives == project.objectives_count {
            panic!("Not all objectives completed");
        }

        // Now, the project is completed
        project.completed = true;

        // Save project
        e.storage().instance().set(&project_key, &project);
    
        // Emitir el evento con el ID del project
        events::project_completed(&e, project_key);

    }

    pub fn cancel_project(e: Env, project_id: u128) {

        // Obtener el proyecto
        let (mut project, project_key) = get_project(&e, project_id);

        // Verificar que la persona que invoca la función es el cliente
        let invoker = get_address(&e, 0);
        if invoker != project.client {
            panic!("Only the client can mark the project as completed");
        }

        // Check if the project is cancelled
        if !project.completed {
            panic!("Project is completed");
        }

        // Check if the project is cancelled
        if !project.cancelled {
            panic!("Project is cancelled");
        }

        // Now, the project is cancelled
        project.cancelled = true;

        // Save project
        e.storage().instance().set(&project_key, &project);

         // Emitir el evento con el ID del project
         events::project_cancelled(&e, project_key);
    }

    pub fn add_objective(e: Env, project_id: u128, prices: Vec<u128>) {

        // Obtener el proyecto
        let (mut project, project_key) = get_project(&e, project_id);

        // Verificar que la persona que invoca la función es el cliente
        let invoker = get_address(&e, 0);
        if invoker != project.client {
            panic!("Only the client can add objectives");
        }

        // Check if the project is cancelled
        if !project.completed {
            panic!("Project is completed");
        }

        // Check if the project is cancelled
        if !project.cancelled {
            panic!("Project is cancelled");
        }
        
         // Iterar sobre los precios y agregar objetivos
        for (i, price) in prices.iter().enumerate() {
            let objective_id = project.objectives_count + i as u128;

            project.objectives.set(objective_id, Objective {
                price: price,
                half_paid: 0,
                completed: false,
            });

            // Emitir el evento con el ID del objetivo
            events::objective_added(&e, project_key.clone(), objective_id, price);
        }

        // Actualizar el recuento de objetivos del proyecto
        project.objectives_count += prices.len() as u128;

        // Guardar el proyecto actualizado en el almacenamiento
        e.storage().instance().set(&project_key, &project);
    }

    pub fn fund_objective(e: Env, project_id: u128, objective_id: u128, usdc_token: Address) {
        
        // Obtener el proyecto
        let (mut project, project_key) = get_project(&e, project_id);

        // Verificar que la persona que invoca la función es el cliente
        let invoker = get_address(&e, 0);
        if invoker != project.client {
            panic!("Only the client can fund objectives");
        }

        // Obtener el objetivo del proyecto
        let mut objective = project.objectives.get(objective_id).unwrap();

        // Verificar que el objetivo no ha sido financiado previamente
        if objective.half_paid > 0 {
            panic!("Objective already funded");
        }

        // Calcular la mitad del precio del objetivo y convertirlo a i128
        let half_price = (objective.price / 2) as i128;

        // Transferir la mitad del precio desde el cliente al contrato
        // Para Stellar, utilizamos el contrato de USDC (aquí simplificado)
        let usdc_client = soroban_sdk::token::Client::new(&e, &usdc_token);
        usdc_client.transfer_from(
            &invoker,  
            &project.client,
            &e.current_contract_address(),
            &half_price       
        );

        // Actualizar el objetivo para reflejar el pago parcial
        objective.half_paid = half_price as u128;
        project.objectives.set(objective_id, objective);

        // Emitir el evento con el ID del objetivo
        events::objective_funded(&e, project_key.clone(), objective_id, half_price as u128);

        // Guardar el proyecto actualizado
        e.storage().instance().set(&project_key, &project);
    }

    pub fn complete_objective(e: Env, project_id: u128, objective_id: u128, usdc_token: Address) {

        // Obtener el proyecto
        let (mut project, project_key) = get_project(&e, project_id);

        // Verificar que la persona que invoca la función es el freelancer
        let invoker = get_address(&e, 0);
        if invoker != project.freelancer {
            panic!("Only the freelancer can complete objectives");
        }

        // Obtener el objetivo del proyecto
        let mut objective = project.objectives.get(objective_id).unwrap();

        // Verificar que el objetivo ha sido financiado parcialmente
        if objective.half_paid == 0 {
            panic!("Objective not funded");
        }

        // Verificar que el objetivo no ha sido completado previamente
        if objective.completed {
            panic!("Objective already completed");
        }

        // Calcular el precio restante del objetivo
        let remaining_price = (objective.price - objective.half_paid) as i128;

        // Transferir el precio restante desde el cliente al contrato
        let usdc_client = soroban_sdk::token::Client::new(&e, &usdc_token);
        usdc_client.transfer_from(
            &project.client,  
            &project.client, // La cuenta fuente es el cliente
            &e.current_contract_address(), // El contrato es el receptor
            &remaining_price
        );

        // Transferir el precio total del objetivo al freelancer
        usdc_client.transfer(
            &e.current_contract_address(), // El contrato transfiere los fondos
            &project.freelancer,           // El freelancer es el receptor
            &(objective.price as i128)     // El precio total del objetivo
        );

        // Marcar el objetivo como completado y actualizar los contadores
        objective.completed = true;
        project.completed_objectives += 1;
        project.earned_amount += objective.price;

        // Actualizar el objetivo en el almacenamiento
        project.objectives.set(objective_id, objective.clone());

        // Emitir el evento con el ID del objetivo
        events::objective_completed(&e, project_key.clone(), objective_id, objective.price);

        // Guardar el proyecto actualizado
        e.storage().instance().set(&project_key, &project);
    }

    pub fn get_projects_by_freelancer(e: Env, freelancer: Address) -> Vec<Project> {
        
        // Obtener todos los proyectos
        let all_projects = get_all_projects(&e);

        // Crear un vector para almacenar los proyectos que pertenecen al freelancer
        let mut result: Vec<Project> = Vec::new(&e);
        let mut index: u32 = 0;

        for i in 0..all_projects.len() {
            // Obtener el proyecto por su índice en el vector
            let project = all_projects.get(i).unwrap(); // Aquí `i` es el índice en el vector

            // Verificar si el proyecto pertenece al freelancer
            if project.freelancer == freelancer {
                result.set(index, project); // Añadir el proyecto al vector resultado
                index += 1;
            }
        }

        result
    }

}

#[cfg(test)]
mod test;