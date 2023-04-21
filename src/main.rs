#![feature(generators)] 

use std::{rc::Rc, cell::Cell, time::Duration};

// Estructuras necesarias para modelar y ejecutar la simulación.
// Simulation:  Estructura principal, encargada de ejecutar la simulación
//              Y mantener todo el state asociado
// Key:      Identifica un generador insertado en simulation
// GenBoxed: Un alias definido como:
//   type GenBoxed<R, C = ()> = Box<dyn Generator<R, Yield = Action, Return = C> + Unpin>
// En palabras sencillas es un puntero inteligente que guarda un generador      
use simulator::{Key, Simulation, GenBoxed, Action, State, StateKey};

fn main() {
    // Instanciar el simulador
    let mut simulation = Simulation::default();
    
    // Obtener acceso al state
    let shared_state = simulation.state();
    
    // Extraer temporalmente el state y dejar un state vacio en su lugar
    // No es posible modificar el state sin hacer este paso.
    let mut state = shared_state.take();

    // Insertar un null temporalmente representando la Key de Entity B
    let entity_b_key = state.insert(None);

    // Insertar una estructura que llevara registro de si Entity A y Entity B estan en Passivate
    let entity_states = state.insert(Passivated { entity_a: false, entity_b: false });

    // Insertar las entidades al simulador
    let a_key = simulation.add_generator(entity_a(Rc::clone(&shared_state), entity_b_key, entity_states));
    let b_key = simulation.add_generator(entity_b(Rc::clone(&shared_state), a_key, entity_states));
    
    // Reemplazar el null por el verdadero valor de las Key de las entidades.
    *state.get_mut(entity_b_key).unwrap() = Some(b_key);

    // Devolver el state al simulador
    // este paso debe hacerse antes de ejecutar la simulacion ej. step_with, step, run_with_limit, etc.
    shared_state.set(state);

    // Agendar las entidades
    simulation.schedule_now(b_key);
    simulation.schedule_now(a_key);
    
    // Avanzar la simulación hasta un maximo de 60 segundos o no queden eventos en la FEL
    simulation.run_with_limit(Duration::from_secs(60));
}

fn entity_a(shared_state: Rc<Cell<State>>, entity_b_key: StateKey<Option<Key>>, entity_states_key: StateKey<Passivated>) -> GenBoxed<()> {
    Box::new(move |_|{
        // Extrae el state actual dejando uno nuevo en su lugar (temporalmente)
        let mut state = shared_state.take();
        // Extrae permanentemente del state el valor asociado a entity_b_key
        let entity_b_key = state.remove(entity_b_key).flatten().unwrap();
        // Devuelve el state
        shared_state.set(state);
        loop {
            // Un HOLD (imaginemos que es random)
            println!("[ENTITY A] -> HOLD");
            yield Action::Hold(Duration::from_secs(5));
            println!("[ENTITY A] <- HOLD");

            // Extrae el state actual dejando uno nuevo en su lugar (temporalmente)
            let mut state = shared_state.take();
            // Extraer el struct Passivated desde el state
            let entity_states = state.get_mut(entity_states_key).unwrap();
            // Si entity_b esta en passivate
            if entity_states.entity_b {
                // Retornar el state
                shared_state.set(state);
                // Enviar un Activate
                println!("[ENTITY A] -> ACTIVATE [ENTITY B]");
                yield Action::ActivateOne(entity_b_key);
            } else {
                // De no estar en Passivate igual debemos devolver el state
                // De lo contrario quedamos en un estado inconsistente (de hecho el codigo no compila sin este else)
                // En una rama devolvemos el state pero en otra no lo cual rust rechaza.
                shared_state.set(state);
            }
            let mut state = shared_state.take();
            state.get_mut(entity_states_key).unwrap().entity_a = true;
            shared_state.set(state);
            println!("[ENTITY A] -> PASSIVATE");
            yield Action::Passivate;
            println!("[ENTITY A] <- PASSIVATE");

            // Al salir de Passivate actualizamos el estado de la entidad en Passivated
            let mut state = shared_state.take();
            state.get_mut(entity_states_key).unwrap().entity_a = false;
            // No olvidar devolver el state antes de un yield
            shared_state.set(state);
        }
    })
}

fn entity_b(shared_state: Rc<Cell<State>>, entity_a_key: Key, entity_states_key: StateKey<Passivated>) -> GenBoxed<()> {
    Box::new(move |_| {
        // Extrae el state actual dejando uno nuevo en su lugar (temporalmente)
        let mut state = shared_state.take();
        // Extraer el struct Passivated desde el state
        let entity_states = state.get_mut(entity_states_key).unwrap();
        // Cambia el valor de entity_b en Passivated a true para indicar que se encuentra en passivate.
        entity_states.entity_b = true;
        // Devuelve el state (olvidar este paso eliminara el state actual y dejara en su lugar uno nuevo)
        // WARNING: Debe hacerse antes de un yield o el proximo generador recibira un state vacio.
        shared_state.set(state);
        // Emite el evento Passivate.
        println!("[ENTITY B] -> PASSIVATE");
        yield Action::Passivate;
        println!("[ENTITY B] <- PASSIVATE");

        // Lo mismo que el inicio pero ahora solo cambiando entity_b a false en Passivated.
        let mut state = shared_state.take();
        state.get_mut(entity_states_key).unwrap().entity_b = false;
        shared_state.set(state);

        loop {
            // Un HOLD (imaginemos que es random)
            println!("[ENTITY B] -> HOLD");
            yield Action::Hold(Duration::from_secs(5));
            println!("[ENTITY B] <- HOLD");

            // Extrae el state actual dejando uno nuevo en su lugar (temporalmente)
            let mut state = shared_state.take();
            // Extraer el struct Passivated desde el state
            let entity_states = state.get_mut(entity_states_key).unwrap();
            // Si entity_a esta en passivate
            if entity_states.entity_a {
                // Retornar el state
                shared_state.set(state);
                // Enviar un Activate
                println!("[ENTITY B] -> ACTIVATE [ENTITY A]");
                yield Action::ActivateOne(entity_a_key);
            } else {
                // De no estar en Passivate igual debemos devolver el state
                // De lo contrario quedamos en un estado inconsistente (de hecho el codigo no compila sin este else)
                // En una rama devolvemos el state pero en otra no lo cual rust rechaza.
                shared_state.set(state);
            }
            let mut state = shared_state.take();
            state.get_mut(entity_states_key).unwrap().entity_b = true;
            shared_state.set(state);
            println!("[ENTITY B] -> PASSIVATE");
            yield Action::Passivate;
            println!("[ENTITY B] <- PASSIVATE");

            // Al salir de Passivate actualizamos el estado de la entidad en Passivated
            let mut state = shared_state.take();
            state.get_mut(entity_states_key).unwrap().entity_b = false;
            // No olvidar devolver el state antes de un yield
            shared_state.set(state);
        }
    })
}

// Estructura auxiliar para determinar si las entidades estan en passivate
pub struct Passivated {
    entity_a: bool,
    entity_b: bool,
}
