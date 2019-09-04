use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateSessionReply {
    pub ret_msg: Option<String>,
    pub session_id: Option<String>,
    pub timestamp: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
pub struct GetMatchIdsByQueueReply {
    pub ret_msg: Option<String>,
    pub Active_Flag: Option<char>,
    pub Match: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
pub struct MergedPlayer {
    pub merge_datetime: String,
    pub playerId: String,
    pub portalId: String,
}

#[allow(non_snake_case)]
#[derive(Clone, Deserialize, Serialize)]
pub struct PlayerMatchDetails {
    pub Account_Level: Option<u8>,
    pub ActiveId1: Option<u16>,
    pub ActiveId2: Option<u16>,
    pub ActiveId3: Option<u16>,
    pub ActiveId4: Option<u16>,
    pub ActivePlayerId: Option<String>,
    pub Assists: Option<u8>,
    pub Ban1: Option<String>,
    pub Ban10: Option<String>,
    pub Ban10Id: Option<u16>,
    pub Ban1Id: Option<u16>,
    pub Ban2: Option<String>,
    pub Ban2Id: Option<u16>,
    pub Ban3: Option<String>,
    pub Ban3Id: Option<u16>,
    pub Ban4: Option<String>,
    pub Ban4Id: Option<u16>,
    pub Ban5: Option<String>,
    pub Ban5Id: Option<u16>,
    pub Ban6: Option<String>,
    pub Ban6Id: Option<u16>,
    pub Ban7: Option<String>,
    pub Ban7Id: Option<u16>,
    pub Ban8: Option<String>,
    pub Ban8Id: Option<u16>,
    pub Ban9: Option<String>,
    pub Ban9Id: Option<u16>,
    pub Camps_Cleared: Option<u8>,
    pub Conquest_Losses: Option<u16>,
    pub Conquest_Points: Option<u16>,
    pub Conquest_Tier: Option<u8>,
    pub Conquest_Wins: Option<u16>,
    pub Damage_Bot: Option<u32>,
    pub Damage_Done_In_Hand: Option<u32>,
    pub Damage_Done_Magical: Option<u32>,
    pub Damage_Done_Physical: Option<u32>,
    pub Damage_Mitigated: Option<u32>,
    pub Damage_Player: Option<u32>,
    pub Damage_Taken: Option<u32>,
    pub Damage_Taken_Magical: Option<u32>,
    pub Damage_Taken_Physical: Option<u32>,
    pub Deaths: Option<u8>,
    pub Distance_Traveled: Option<u32>,
    pub Duel_Losses: Option<u16>,
    pub Duel_Points: Option<u16>,
    pub Duel_Tier: Option<u8>,
    pub Duel_Wins: Option<u16>,
    pub Entry_Datetime: Option<String>,
    pub Final_Match_Level: Option<u8>,
    pub First_Ban_Side: Option<String>,
    pub GodId: Option<u16>,
    pub Gold_Earned: Option<u32>,
    pub Gold_Per_Minute: Option<u16>,
    pub Healing: Option<u32>,
    pub Healing_Bot: Option<u32>,
    pub Healing_Player_Self: Option<u32>,
    pub ItemId1: Option<u16>,
    pub ItemId2: Option<u16>,
    pub ItemId3: Option<u16>,
    pub ItemId4: Option<u16>,
    pub ItemId5: Option<u16>,
    pub ItemId6: Option<u16>,
    pub Item_Active_1: Option<String>,
    pub Item_Active_2: Option<String>,
    pub Item_Active_3: Option<String>,
    pub Item_Active_4: Option<String>,
    pub Item_Purch_1: Option<String>,
    pub Item_Purch_2: Option<String>,
    pub Item_Purch_3: Option<String>,
    pub Item_Purch_4: Option<String>,
    pub Item_Purch_5: Option<String>,
    pub Item_Purch_6: Option<String>,
    pub Joust_Losses: Option<u16>,
    pub Joust_Points: Option<u16>,
    pub Joust_Tier: Option<u8>,
    pub Joust_Wins: Option<u16>,
    pub Killing_Spree: Option<u8>,
    pub Kills_Bot: Option<u16>,
    pub Kills_Double: Option<u8>,
    pub Kills_Fire_Giant: Option<u8>,
    pub Kills_First_Blood: Option<u8>,
    pub Kills_Gold_Fury: Option<u8>,
    pub Kills_Penta: Option<u8>,
    pub Kills_Phoenix: Option<u8>,
    pub Kills_Player: Option<u8>,
    pub Kills_Quadra: Option<u8>,
    pub Kills_Siege_Juggernaut: Option<u8>,
    pub Kills_Single: Option<u8>,
    pub Kills_Triple: Option<u8>,
    pub Kills_Wild_Juggernaut: Option<u8>,
    pub Map_Game: Option<String>,
    pub Mastery_Level: Option<u8>,
    pub Match: Option<u32>,
    pub Match_Duration: Option<u64>,
    pub MergedPlayers: Option<Vec<MergedPlayer>>,
    pub Minutes: Option<u8>,
    pub Multi_kill_Max: Option<u8>,
    pub Objective_Assists: Option<u8>,
    pub PartyId: Option<u32>,
    pub Rank_Stat_Conquest: Option<f32>,
    pub Rank_Stat_Duel: Option<f32>,
    pub Rank_Stat_Joust: Option<f32>,
    pub Reference_Name: Option<String>,
    pub Region: Option<String>,
    pub Skin: Option<String>,
    pub SkinId: Option<u16>,
    pub Structure_Damage: Option<u16>,
    pub Surrendered: Option<u8>,
    pub TaskForce: Option<u8>,
    pub Team1Score: Option<u64>,
    pub Team2Score: Option<u64>,
    pub TeamId: Option<u32>,
    pub Team_Name: Option<String>,
    pub Time_In_Match_Seconds: Option<u16>,
    pub Towers_Destroyed: Option<u8>,
    pub Wards_Placed: Option<u16>,
    pub Win_Status: Option<String>,
    pub Winning_TaskForce: Option<u8>,
    pub hasReplay: Option<char>,
    pub hz_gamer_tag: Option<String>,
    pub hz_player_name: Option<String>,
    pub match_queue_id: Option<u16>,
    pub name: Option<String>,
    pub playerId: Option<String>,
    pub playerName: Option<String>,
    pub playerPortalId: Option<String>,
    pub playerPortalUserId: Option<String>,
    pub ret_msg: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Serialize)]
pub struct God {
    pub id: u16,
    pub Name: String,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Serialize)]
pub struct Item {
    pub ItemId: u16,
    pub DeviceName: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn get_size_of_struct() {
        println!("{}", std::mem::size_of::<PlayerMatchDetails>());
    }
}
