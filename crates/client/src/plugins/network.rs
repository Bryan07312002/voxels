use bevy::prelude::*;
use net::ClientChannel;
use core_types::ViewDistance;

#[derive(Resource)]
pub struct NetworkClient(pub ClientChannel);

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        // Bind the UDP socket once when the app starts up
        match ClientChannel::new("127.0.0.1:25565") {
            Ok(client) => {
                info!("Network client successfully bound to server.");
                
                // Send the initial connection handshake right away
                let _ = client.send_connect(ViewDistance(12)); // Config-driven or default view distance
                
                app.insert_resource(NetworkClient(client));
            }
            Err(e) => {
                error!("Failed to bind network client socket: {e}");
            }
        }
    }
}
