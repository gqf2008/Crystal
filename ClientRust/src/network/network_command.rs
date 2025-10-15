// 网络命令 - UI线程发送给网络线程的命令

use mir2_shared::packets::client;

/// Commands that can be sent from UI thread to network thread
#[derive(Debug, Clone)]
pub enum NetworkCommand {
    /// Send login packet
    Login {
        username: String,
        password: String,
    },
    
    /// Create new account
    NewAccount {
        account_id: String,
        password: String,
        email: String,
        username: String,
        secret_question: String,
        secret_answer: String,
    },
    
    /// Change password
    ChangePassword {
        account_id: String,
        current_password: String,
        new_password: String,
    },
    
    /// Select character
    SelectCharacter {
        index: i32,
    },
    
    /// Create new character
    NewCharacter {
        name: String,
        class: u8,
        gender: u8,
    },
    
    /// Delete character
    DeleteCharacter {
        index: i32,
    },
    
    /// Start game with selected character
    StartGame {
        character_index: i32,
    },
    
    /// Send movement command (Walk/Run)
    Move {
        direction: u8,  // MirDirection as u8
        location: (i32, i32),  // (x, y)
    },
    
    /// Disconnect from server
    Disconnect,
}
