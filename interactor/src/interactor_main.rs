#![allow(non_snake_case)]

mod proxy;

use async_std::task;
use multiversx_sc_snippets::imports::*;
use multiversx_sc_snippets::multiversx_sc_scenario::api::VMHooksApi;
use multiversx_sc_snippets::multiversx_sc_scenario::scenario_model::Esdt;
use multiversx_sc_snippets::sdk;
use mvx_game_sc::game_proxy::{GameSettings, Status};
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};
use std::env::consts::EXE_EXTENSION;
use std::result;
use std::time::Duration;
use std::{
    io::{Read, Write},
    path::Path,
};

const GATEWAY: &str = sdk::gateway::DEVNET_GATEWAY;
const STATE_FILE: &str = "state.toml";
const TOKEN_ID: &str = "VLD-76ecd8";
const ANOTHER_TOKEN_ID: &str = "WRNG-67975d";
const THIRD_TOKEN_ID: &str = "RAND-e3641c";
const INVALID_TOKEN_ID: &str = "123";
const FEE_AMOUNT: u64 = 1u64;
const WAGE_AMOUNT: u64 = 1u64;
const WAITING_TIME: u64 = 100u64;
const OWNER_ADDR: &str = "erd1r6f7nfpyzul2tef7gne5h6nx9xqnyt5gehwltlxymnqkztjjzvuqdhderc";
const SECOND_USER_ADDR: &str = "erd1tjusdv806tuwzllgesljglm7y9jef38wdylkvp85v7a46z9x23us0z5xtr";
const THIRD_USER_ADDR: &str = "erd1pu4r9rxgn8f7a7gwjchtxjz6y4u3ha7fy93w6r3fjeq26jaqkqjs4ly8fd";

pub struct GameSettingsMock {
    pub time_limit: u64,            //start_time + waiting time
    pub number_of_players_min: u64, //min and max
    pub number_of_players_max: u64,
    pub wager: u64,
    pub creator: Bech32Address,
    pub status: proxy::Status,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut args = std::env::args();
    let _ = args.next();
    let cmd = args.next().expect("at least one argument required");
    let mut interact = ContractInteract::new().await;
    match cmd.as_str() {
        // "deploy" => interact.deploy().await,
        // "createGame" => interact.create_game().await,
        // "joinGame" => interact.join_game().await,
        // "claimBackWager" => interact.claim_back_wager().await,
        // "getTokenId" => interact.token_id().await,
        // "getGameStartFee" => interact.game_start_fee().await,
        // "getEnabled" => interact.enabled().await,
        // "isUserAdmin" => interact.is_user_admin().await,
        // "getLastGameId" => interact.last_game_id().await,
        // "getGameSettings" => interact.game_settings().await,
        // "getGameIdBySettings" => interact.game_id().await,
        // "getPlayers" => interact.players().await,
        // "getGamesPerUser" => interact.games_per_user().await,
        // "sendReward" => interact.send_reward().await,
        // "enableSC" => interact.enable_sc().await,
        // "disableSC" => interact.disable_sc().await,
        // "setTokenId" => interact.set_token_id().await,
        // "setGameStartFee" => interact.set_game_start_fee().await,
        // "setAdmin" => interact.set_admin().await,
        // "removeAdmin" => interact.remove_admin().await,
        _ => panic!("unknown command: {}", &cmd),
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    contract_address: Option<Bech32Address>,
}

impl State {
    // Deserializes state from file
    pub fn load_state() -> Self {
        if Path::new(STATE_FILE).exists() {
            let mut file = std::fs::File::open(STATE_FILE).unwrap();
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();
            toml::from_str(&content).unwrap()
        } else {
            Self::default()
        }
    }

    /// Sets the contract address
    pub fn set_address(&mut self, address: Bech32Address) {
        self.contract_address = Some(address);
    }

    /// Returns the contract address
    pub fn current_address(&self) -> &Bech32Address {
        self.contract_address
            .as_ref()
            .expect("no known contract, deploy first")
    }
}

impl Drop for State {
    // Serializes state to file
    fn drop(&mut self) {
        let mut file = std::fs::File::create(STATE_FILE).unwrap();
        file.write_all(toml::to_string(self).unwrap().as_bytes())
            .unwrap();
    }
}

struct ContractInteract {
    interactor: Interactor,
    owner_address: Address,
    second_user: Address,
    third_user: Address,
    contract_code: BytesValue,
    state: State,
}

impl ContractInteract {
    async fn new() -> Self {
        let mut interactor = Interactor::new(GATEWAY).await;
        let owner_address = interactor
            .register_wallet(Wallet::from_pem_file("wallet1.pem").expect("wallet not found"));
        let second_user = interactor
            .register_wallet(Wallet::from_pem_file("wallet2.pem").expect("wallet not found"));
        let third_user = interactor
            .register_wallet(Wallet::from_pem_file("wallet3.pem").expect("wallet not found"));

        let contract_code = BytesValue::interpret_from(
            "mxsc:../output/mvx-game-sc.mxsc.json",
            &InterpreterContext::default(),
        );

        ContractInteract {
            interactor,
            owner_address,
            second_user,
            third_user,
            contract_code,
            state: State::load_state(),
        }
    }

    async fn deploy_basic(&mut self) {
        let enabled_opt = OptionalValue::Some(bool::default());
        let game_start_fee_opt = OptionalValue::Some(BigUint::<StaticApi>::from(0u128));
        let token_id_opt =
            OptionalValue::Some(EgldOrEsdtTokenIdentifier::esdt(TOKEN_ID.as_bytes()));

        let new_address = self
            .interactor
            .tx()
            .from(&self.owner_address)
            .gas(50_000_000u64)
            .typed(proxy::MvxGameScProxy)
            .init(enabled_opt, game_start_fee_opt, token_id_opt)
            .code(&self.contract_code)
            .returns(ReturnsNewAddress)
            .prepare_async()
            .run()
            .await;
        let new_address_bech32 = bech32::encode(&new_address);
        self.state.set_address(Bech32Address::from_bech32_string(
            new_address_bech32.clone(),
        ));

        println!("new address: {new_address_bech32}");
    }

    async fn deploy(
        &mut self,
        enabled_opt: OptionalValue<bool>,
        game_start_fee_opt: OptionalValue<BigUint<StaticApi>>,
    ) {
        let token_id_opt =
            OptionalValue::Some(EgldOrEsdtTokenIdentifier::esdt(TOKEN_ID.as_bytes()));

        let new_address = self
            .interactor
            .tx()
            .from(&self.owner_address)
            .gas(50_000_000)
            .typed(proxy::MvxGameScProxy)
            .init(enabled_opt, game_start_fee_opt, token_id_opt)
            .code(&self.contract_code)
            .returns(ReturnsNewAddress)
            .prepare_async()
            .run()
            .await;
        let new_address_bech32 = bech32::encode(&new_address);
        self.state.set_address(Bech32Address::from_bech32_string(
            new_address_bech32.clone(),
        ));

        println!("new address: {new_address_bech32}");
    }

    async fn deploy_fail(
        &mut self,
        enabled_opt: OptionalValue<bool>,
        game_start_fee_opt: OptionalValue<BigUint<StaticApi>>,
        expected_result: ExpectError<'_>,
    ) {
        let token_id_opt =
            OptionalValue::Some(EgldOrEsdtTokenIdentifier::esdt(TOKEN_ID.as_bytes()));

        self.interactor
            .tx()
            .from(&self.owner_address)
            .gas(50_000_000)
            .typed(proxy::MvxGameScProxy)
            .init(enabled_opt, game_start_fee_opt, token_id_opt)
            .code(&self.contract_code)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;
    }

    async fn deploy_fail_no_token_id(
        &mut self,
        enabled_opt: OptionalValue<bool>,
        game_start_fee_opt: OptionalValue<BigUint<StaticApi>>,
        expected_result: ExpectError<'_>,
    ) {
        self.interactor
            .tx()
            .from(&self.owner_address)
            .gas(50_000_000)
            .typed(proxy::MvxGameScProxy)
            .init(
                enabled_opt,
                game_start_fee_opt,
                OptionalValue::<EgldOrEsdtTokenIdentifier<_>>::None,
            )
            .code(&self.contract_code)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;
    }

    async fn create_game(
        &mut self,
        game_creator: &Bech32Address,
        token_id: &str,
        token_nonce: u64,
        token_amount: u128,
        waiting_time: u64,
        number_of_players_min: u64,
        number_of_players_max: u64,
        wager: u128,
    ) -> u64 {
        let token_amount = BigUint::<StaticApi>::from(token_amount);

        let wager = BigUint::<StaticApi>::from(wager);

        let response = self
            .interactor
            .tx()
            .from(game_creator)
            .to(self.state.current_address())
            .gas(80_000_000u64)
            .typed(proxy::MvxGameScProxy)
            .create_game(
                waiting_time,
                number_of_players_min,
                number_of_players_max,
                wager,
            )
            .payment((TokenIdentifier::from(token_id), token_nonce, token_amount))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        response
    }

    async fn create_game_fail(
        &mut self,
        game_creator: &Bech32Address,
        token_id: &str,
        token_nonce: u64,
        token_amount: u128,
        waiting_time: u64,
        number_of_players_min: u64,
        number_of_players_max: u64,
        wager: u128,
        expected_result: ExpectError<'_>,
    ) {
        let token_amount = BigUint::<StaticApi>::from(token_amount);

        let wager = BigUint::<StaticApi>::from(wager);

        self.interactor
            .tx()
            .from(game_creator)
            .to(self.state.current_address())
            .gas(80_000_000u64)
            .typed(proxy::MvxGameScProxy)
            .create_game(
                waiting_time,
                number_of_players_min,
                number_of_players_max,
                wager,
            )
            .payment((TokenIdentifier::from(token_id), token_nonce, token_amount))
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;
    }

    async fn join_game(
        &mut self,
        joiner: &Bech32Address,
        token_id: &str,
        token_nonce: u64,
        token_amount: u128,
        game_id: u64,
    ) {
        let token_id = TokenIdentifier::from(token_id);
        let token_amount = BigUint::<StaticApi>::from(token_amount);

        let _response = self
            .interactor
            .tx()
            .from(joiner)
            .to(self.state.current_address())
            .gas(80_000_000u64)
            .typed(proxy::MvxGameScProxy)
            .join_game(game_id)
            .payment((TokenIdentifier::from(token_id), token_nonce, token_amount))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;
    }

    async fn join_game_fail(
        &mut self,
        joiner: &Bech32Address,
        token_id: &str,
        token_nonce: u64,
        token_amount: u128,
        game_id: u64,
        expected_result: ExpectError<'_>,
    ) {
        let token_id = TokenIdentifier::from(token_id);
        let token_amount = BigUint::<StaticApi>::from(token_amount);

        let _response = self
            .interactor
            .tx()
            .from(joiner)
            .to(self.state.current_address())
            .gas(80_000_000u64)
            .typed(proxy::MvxGameScProxy)
            .join_game(game_id)
            .payment((TokenIdentifier::from(token_id), token_nonce, token_amount))
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;
    }

    async fn claim_back_wager(&mut self, claimer: &Bech32Address, game_id: u64) {
        let response = self
            .interactor
            .tx()
            .from(claimer)
            .to(self.state.current_address())
            .gas(80_000_000u64)
            .typed(proxy::MvxGameScProxy)
            .claim_back_wager(game_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;
    }

    async fn claim_back_wager_fail(
        &mut self,
        claimer: &Bech32Address,
        game_id: u64,
        expected_result: ExpectError<'_>,
    ) {
        let response = self
            .interactor
            .tx()
            .from(claimer)
            .to(self.state.current_address())
            .gas(80_000_000u64)
            .typed(proxy::MvxGameScProxy)
            .claim_back_wager(game_id)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;
    }

    async fn token_id(&mut self, token_id: &[u8]) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::MvxGameScProxy)
            .token_id()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        assert_eq!(result_value, EgldOrEsdtTokenIdentifier::esdt(token_id));
    }

    async fn game_start_fee(&mut self) -> RustBigUint {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::MvxGameScProxy)
            .game_start_fee()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
        result_value
    }

    async fn enabled(&mut self) -> bool {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::MvxGameScProxy)
            .enabled()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        result_value
    }

    async fn is_user_admin(&mut self, user: &str) -> bool {
        let user = bech32::decode(user);

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::MvxGameScProxy)
            .is_user_admin(user)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        result_value
    }

    async fn last_game_id(&mut self) -> u64 {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::MvxGameScProxy)
            .last_game_id()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        result_value
    }

    async fn game_settings(&mut self, game_id: u64) -> GameSettingsMock {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::MvxGameScProxy)
            .game_settings(game_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        GameSettingsMock {
            time_limit: result_value.time_limit,
            number_of_players_min: result_value.number_of_players_min,
            number_of_players_max: result_value.number_of_players_max,
            wager: BigUint::<StaticApi>::to_u64(&result_value.wager).expect("e greu"),
            creator: result_value.creator.to_address().into(),
            status: result_value.status,
        }
    }

    async fn game_id(&mut self) {
        let game_settings = proxy::GameSettings {
            time_limit: 0u64,
            number_of_players_min: 0u64,
            number_of_players_max: 0u64,
            wager: BigUint::<StaticApi>::from(0u128),
            creator: ManagedAddress::from_address(&Address::from_slice(
                SECOND_USER_ADDR.as_bytes(),
            )),
            status: proxy::Status::Invalid,
        };

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::MvxGameScProxy)
            .game_id(proxy::GameSettings::from(game_settings))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn players(&mut self) {
        let game_id = 0u64;

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::MvxGameScProxy)
            .players(game_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn games_per_user(&mut self) {
        let user = bech32::decode("");

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::MvxGameScProxy)
            .games_per_user(user)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn send_reward(&mut self, sender: &Bech32Address, game_id: u64) {
        let winners = OptionalValue::Some(MultiValueVec::from(vec![
            (
                ManagedAddress::from_address(&Address::from_slice(SECOND_USER_ADDR.as_bytes())),
                0u64,
            ),
            (
                ManagedAddress::from_address(&Address::from_slice(THIRD_USER_ADDR.as_bytes())),
                0u64,
            ),
        ]));

        let response = self
            .interactor
            .tx()
            .from(sender)
            .to(self.state.current_address())
            .gas(80_000_000u64)
            .typed(proxy::MvxGameScProxy)
            .send_reward(game_id, winners)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn send_reward_fail(
        &mut self,
        sender: &Bech32Address,
        game_id: u64,
        expected_result: ExpectError<'_>,
    ) {
        let winners = OptionalValue::Some(MultiValueVec::from(vec![
            (
                ManagedAddress::from_address(&Address::from_slice(SECOND_USER_ADDR.as_bytes())),
                0u64,
            ),
            (
                ManagedAddress::from_address(&Address::from_slice(THIRD_USER_ADDR.as_bytes())),
                0u64,
            ),
        ]));

        let response = self
            .interactor
            .tx()
            .from(sender)
            .to(self.state.current_address())
            .gas(80_000_000u64)
            .typed(proxy::MvxGameScProxy)
            .send_reward(game_id, winners)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn enable_sc(&mut self) {
        let response = self
            .interactor
            .tx()
            .from(&self.owner_address)
            .to(self.state.current_address())
            .gas(80_000_000u64)
            .typed(proxy::MvxGameScProxy)
            .enable_sc()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn disable_sc(&mut self) {
        let response = self
            .interactor
            .tx()
            .from(&self.owner_address)
            .to(self.state.current_address())
            .gas(80_000_000u64)
            .typed(proxy::MvxGameScProxy)
            .disable_sc()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn set_token_id(&mut self, sender: &Bech32Address, token_id: &str) {
        let token_id = EgldOrEsdtTokenIdentifier::esdt(token_id);

        let _response = self
            .interactor
            .tx()
            .from(sender)
            .to(self.state.current_address())
            .gas(80_000_000u64)
            .typed(proxy::MvxGameScProxy)
            .set_token_id(token_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;
    }

    async fn set_token_id_fail(
        &mut self,
        sender: &Bech32Address,
        token_id: &str,
        expected_result: ExpectError<'_>,
    ) {
        let token_id = EgldOrEsdtTokenIdentifier::esdt(token_id);

        let _response = self
            .interactor
            .tx()
            .from(sender)
            .to(self.state.current_address())
            .gas(80_000_000u64)
            .typed(proxy::MvxGameScProxy)
            .set_token_id(token_id)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;
    }

    async fn set_game_start_fee(&mut self, sender: &Bech32Address, amount: u128) {
        let amount = BigUint::<StaticApi>::from(amount);

        let _response = self
            .interactor
            .tx()
            .from(sender)
            .to(self.state.current_address())
            .gas(80_000_000u64)
            .typed(proxy::MvxGameScProxy)
            .set_game_start_fee(amount)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;
    }

    async fn set_game_start_fee_fail(
        &mut self,
        sender: &Bech32Address,
        amount: u128,
        expected_result: ExpectError<'_>,
    ) {
        let amount = BigUint::<StaticApi>::from(amount);

        let _response = self
            .interactor
            .tx()
            .from(sender)
            .to(self.state.current_address())
            .gas(80_000_000u64)
            .typed(proxy::MvxGameScProxy)
            .set_game_start_fee(amount)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;
    }

    async fn set_admin(&mut self, user: &str) {
        let user = bech32::decode(user);

        let response = self
            .interactor
            .tx()
            .from(&self.owner_address)
            .to(self.state.current_address())
            .gas(80_000_000u64)
            .typed(proxy::MvxGameScProxy)
            .set_admin(user)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn remove_admin(&mut self, user: &str) {
        let user = bech32::decode(user);

        let response = self
            .interactor
            .tx()
            .from(&self.owner_address)
            .to(self.state.current_address())
            .gas(80_000_000u64)
            .typed(proxy::MvxGameScProxy)
            .remove_admin(user)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }
}

#[tokio::test]
async fn test_deploy() {
    let mut interact = ContractInteract::new().await;
    interact
        .deploy(
            OptionalValue::Some(false),
            OptionalValue::Some(BigUint::<StaticApi>::from(FEE_AMOUNT)),
        )
        .await;

    let game_start_fee = interact.game_start_fee().await;
    assert_eq!(game_start_fee, RustBigUint::from(FEE_AMOUNT));

    interact.token_id(TOKEN_ID.as_bytes()).await;

    let is_enabled = interact.enabled().await;
    assert_eq!(is_enabled, true);
}

// fails
#[tokio::test]
async fn test_deploy_game_start_fee_not_set() {
    let mut interact = ContractInteract::new().await;
    interact
        .deploy_fail(
            OptionalValue::Some(true),
            OptionalValue::None,
            ExpectError(4, "game start fee not set"),
        )
        .await;
}

#[tokio::test]
async fn test_deploy_with_args() {
    let mut interact = ContractInteract::new().await;
    interact
        .deploy_fail_no_token_id(
            OptionalValue::Some(true),
            OptionalValue::Some(BigUint::<StaticApi>::from(FEE_AMOUNT)),
            ExpectError(4, "fee token id not set"),
        )
        .await;
}

#[tokio::test]
async fn test_create_game() {
    let mut interact = ContractInteract::new().await;
    interact
        .deploy(
            OptionalValue::Some(true),
            OptionalValue::Some(BigUint::<StaticApi>::from(FEE_AMOUNT)),
        )
        .await;

    let game_start_fee = interact.game_start_fee().await;
    assert_eq!(game_start_fee, RustBigUint::from(FEE_AMOUNT));

    interact.token_id(TOKEN_ID.as_bytes()).await;

    let is_enabled = interact.enabled().await;
    assert_eq!(is_enabled, true);

    interact.disable_sc().await;

    let is_enabled = interact.enabled().await;
    assert_eq!(is_enabled, false);

    interact
        .create_game_fail(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            FEE_AMOUNT.into(),
            0u64,
            0u64,
            0u64,
            0u128,
            ExpectError(4, "maintenance"),
        )
        .await;

    interact.enable_sc().await;

    let is_enabled = interact.enabled().await;
    assert_eq!(is_enabled, true);

    interact
        .create_game_fail(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            FEE_AMOUNT.into(),
            0u64,
            0u64,
            0u64,
            0u128,
            ExpectError(4, "wager can't be 0"),
        )
        .await;

    interact
        .create_game_fail(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            FEE_AMOUNT.into(),
            0u64,
            0u64,
            0u64,
            WAGE_AMOUNT.into(),
            ExpectError(4, "waiting time can't be 0"),
        )
        .await;

    interact
        .create_game_fail(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            ANOTHER_TOKEN_ID,
            0u64,
            FEE_AMOUNT.into(),
            1u64,
            0u64,
            0u64,
            WAGE_AMOUNT.into(),
            ExpectError(4, "wrong token id"),
        )
        .await;

    interact
        .create_game_fail(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            (FEE_AMOUNT + 1).into(),
            1u64,
            0u64,
            0u64,
            (WAGE_AMOUNT + 1).into(),
            ExpectError(4, "start game payment amount not right"),
        )
        .await;

    interact
        .create_game_fail(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            FEE_AMOUNT.into(),
            1u64,
            0u64,
            1u64,
            WAGE_AMOUNT.into(),
            ExpectError(4, "number of players cannot be 0"),
        )
        .await;

    interact
        .create_game_fail(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            FEE_AMOUNT.into(),
            1u64,
            1u64,
            0u64,
            WAGE_AMOUNT.into(),
            ExpectError(4, "number of players cannot be 0"),
        )
        .await;

    let game_id = interact
        .create_game(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            FEE_AMOUNT.into(),
            1u64,
            1u64,
            1u64,
            WAGE_AMOUNT.into(),
        )
        .await;

    let game_settings = interact.game_settings(game_id).await;

    assert_eq!(game_settings.number_of_players_min, 1u64);
    assert_eq!(game_settings.number_of_players_max, 1u64);
    assert_eq!(game_settings.wager, WAGE_AMOUNT);
    assert_eq!(
        game_settings.creator,
        Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string())
    );
    assert_eq!(game_settings.status, proxy::Status::Invalid);
}

#[tokio::test]
async fn test_join_game() {
    let mut interact = ContractInteract::new().await;
    interact
        .deploy(
            OptionalValue::Some(true),
            OptionalValue::Some(BigUint::<StaticApi>::from(FEE_AMOUNT)),
        )
        .await;

    let game_id = interact
        .create_game(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            FEE_AMOUNT.into(),
            1u64,
            1u64,
            1u64,
            WAGE_AMOUNT.into(),
        )
        .await;

    interact.disable_sc().await;

    interact
        .join_game_fail(
            &Bech32Address::from_bech32_string(THIRD_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            WAGE_AMOUNT.into(),
            game_id,
            ExpectError(4, "maintenance"),
        )
        .await;

    interact.enable_sc().await;

    let games_length = interact.last_game_id().await;

    interact
        .join_game_fail(
            &Bech32Address::from_bech32_string(THIRD_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            WAGE_AMOUNT.into(),
            games_length + 1,
            ExpectError(4, "no settings for game id"),
        )
        .await;

    interact
        .join_game_fail(
            &Bech32Address::from_bech32_string(THIRD_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            WAGE_AMOUNT.into(),
            game_id,
            ExpectError(4, "waiting time has passed"),
        )
        .await;

    let game_id = interact
        .create_game(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            FEE_AMOUNT.into(),
            WAITING_TIME,
            1u64,
            1u64,
            WAGE_AMOUNT.into(),
        )
        .await;

    interact
        .join_game(
            &Bech32Address::from_bech32_string(OWNER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            WAGE_AMOUNT.into(),
            game_id,
        )
        .await;

    interact
        .join_game_fail(
            &Bech32Address::from_bech32_string(THIRD_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            WAGE_AMOUNT.into(),
            game_id,
            ExpectError(4, "max number of players reached"),
        )
        .await;

    let game_id = interact
        .create_game(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            FEE_AMOUNT.into(),
            WAITING_TIME * 2,
            1u64,
            1u64,
            WAGE_AMOUNT.into(),
        )
        .await;

    interact
        .join_game_fail(
            &Bech32Address::from_bech32_string(THIRD_USER_ADDR.to_string()),
            THIRD_TOKEN_ID,
            0u64,
            WAGE_AMOUNT.into(),
            game_id,
            ExpectError(4, "wrong token sent"),
        )
        .await;

    interact
        .join_game_fail(
            &Bech32Address::from_bech32_string(THIRD_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            (WAGE_AMOUNT + 1).into(),
            game_id,
            ExpectError(4, "wrong amount paid"),
        )
        .await;

    interact
        .join_game(
            &Bech32Address::from_bech32_string(THIRD_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            WAGE_AMOUNT.into(),
            game_id,
        )
        .await;

    let game_settings = interact.game_settings(game_id).await;

    assert_eq!(game_settings.status, proxy::Status::Valid);

    interact
        .join_game_fail(
            &Bech32Address::from_bech32_string(THIRD_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            WAGE_AMOUNT.into(),
            game_id,
            ExpectError(4, "user already joined this game"),
        )
        .await;
}

#[tokio::test]
async fn test_claim_back_wager() {
    let mut interact = ContractInteract::new().await;

    interact
        .deploy(
            OptionalValue::Some(true),
            OptionalValue::Some(BigUint::<StaticApi>::from(FEE_AMOUNT)),
        )
        .await;

    interact.disable_sc().await;

    interact
        .claim_back_wager_fail(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            0u64,
            ExpectError(4, "maintenance"),
        )
        .await;

    interact.enable_sc().await;

    interact
        .claim_back_wager_fail(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            69u64,
            ExpectError(4, "no settings for game id"),
        )
        .await;

    let game_id = interact
        .create_game(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            FEE_AMOUNT.into(),
            WAITING_TIME * 2,
            1u64,
            1u64,
            WAGE_AMOUNT.into(),
        )
        .await;

    interact
        .claim_back_wager_fail(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            game_id,
            ExpectError(4, "caller has not joined the game"),
        )
        .await;

    interact
        .join_game(
            &Bech32Address::from_bech32_string(THIRD_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            WAGE_AMOUNT.into(),
            game_id,
        )
        .await;

    interact
        .claim_back_wager_fail(
            &Bech32Address::from_bech32_string(THIRD_USER_ADDR.to_string()),
            game_id,
            ExpectError(4, "waiting time is not over yet"),
        )
        .await;

    task::sleep(Duration::from_secs(120)).await;

    interact
        .claim_back_wager_fail(
            &Bech32Address::from_bech32_string(THIRD_USER_ADDR.to_string()),
            game_id,
            ExpectError(
                4,
                "can manually claim back wager only if the game is invalid",
            ),
        )
        .await;

    let game_id = interact
        .create_game(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            FEE_AMOUNT.into(),
            WAITING_TIME,
            2u64,
            2u64,
            WAGE_AMOUNT.into(),
        )
        .await;

    interact
        .join_game(
            &Bech32Address::from_bech32_string(THIRD_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            WAGE_AMOUNT.into(),
            game_id,
        )
        .await;

    task::sleep(Duration::from_secs(100)).await;

    interact
        .claim_back_wager(
            &Bech32Address::from_bech32_string(THIRD_USER_ADDR.to_string()),
            game_id,
        )
        .await;
}

#[tokio::test]
async fn test_owner_small_funcs() {
    let mut interact = ContractInteract::new().await;

    interact
        .deploy(
            OptionalValue::Some(true),
            OptionalValue::Some(BigUint::<StaticApi>::from(FEE_AMOUNT)),
        )
        .await;

    let _game_id = interact
        .create_game(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            FEE_AMOUNT.into(),
            WAITING_TIME,
            2u64,
            2u64,
            WAGE_AMOUNT.into(),
        )
        .await;

    interact
        .set_token_id(
            &Bech32Address::from_bech32_string(OWNER_ADDR.to_string()),
            ANOTHER_TOKEN_ID,
        )
        .await;

    interact
        .create_game_fail(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            FEE_AMOUNT.into(),
            WAITING_TIME,
            2u64,
            2u64,
            WAGE_AMOUNT.into(),
            ExpectError(4, "wrong token id"),
        )
        .await;

    interact
        .set_token_id(
            &Bech32Address::from_bech32_string(OWNER_ADDR.to_string()),
            TOKEN_ID,
        )
        .await;

    interact
        .create_game(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            FEE_AMOUNT.into(),
            WAITING_TIME,
            2u64,
            2u64,
            WAGE_AMOUNT.into(),
        )
        .await;

    interact
        .set_token_id_fail(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            ANOTHER_TOKEN_ID,
            ExpectError(4, "Endpoint can only be called by owner"),
        )
        .await;

    interact.set_admin(SECOND_USER_ADDR).await;

    let is_second_user_admin = interact.is_user_admin(SECOND_USER_ADDR).await;

    assert_eq!(is_second_user_admin, true);

    interact.remove_admin(SECOND_USER_ADDR).await;

    let is_second_user_admin = interact.is_user_admin(SECOND_USER_ADDR).await;

    assert_eq!(is_second_user_admin, false);

    interact
        .set_token_id_fail(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            ANOTHER_TOKEN_ID,
            ExpectError(4, "Endpoint can only be called by owner"),
        )
        .await;

    interact
        .set_game_start_fee(
            &Bech32Address::from_bech32_string(OWNER_ADDR.to_string()),
            (FEE_AMOUNT * 2).into(),
        )
        .await;

    let game_start_fee = interact.game_start_fee().await;

    assert_eq!(game_start_fee, RustBigUint::from(FEE_AMOUNT * 2));

    interact
        .set_game_start_fee_fail(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            (FEE_AMOUNT * 2).into(),
            ExpectError(4, "Endpoint can only be called by owner"),
        )
        .await;
}

#[tokio::test]
async fn test_full_func() {
    let mut interact = ContractInteract::new().await;

    interact
        .deploy(
            OptionalValue::Some(true),
            OptionalValue::Some(BigUint::<StaticApi>::from(FEE_AMOUNT)),
        )
        .await;

    interact.disable_sc().await;

    interact
        .send_reward_fail(
            &Bech32Address::from_bech32_string(OWNER_ADDR.to_string()),
            400u64,
            ExpectError(4, "maintenance"),
        )
        .await;

    interact.enable_sc().await;

    interact
        .send_reward_fail(
            &Bech32Address::from_bech32_string(OWNER_ADDR.to_string()),
            400u64,
            ExpectError(4, "Item not whitelisted"),
        )
        .await;

    interact
        .send_reward_fail(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            400u64,
            ExpectError(4, "Item not whitelisted"),
        )
        .await;

    interact.set_admin(SECOND_USER_ADDR).await;

    interact
        .send_reward_fail(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            400u64,
            ExpectError(4, "no settings for game id"),
        )
        .await;

    let game_id = interact
        .create_game(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            FEE_AMOUNT.into(),
            WAITING_TIME,
            2u64,
            2u64,
            WAGE_AMOUNT.into(),
        )
        .await;

    interact
        .send_reward_fail(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            game_id,
            ExpectError(4, "waiting time is not over yet"),
        )
        .await;

    interact
        .join_game(
            &Bech32Address::from_bech32_string(THIRD_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            WAGE_AMOUNT.into(),
            game_id,
        )
        .await;

    task::sleep(Duration::from_secs(100)).await;

    interact
        .send_reward(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            game_id,
        )
        .await;

    interact
        .claim_back_wager_fail(
            &Bech32Address::from_bech32_string(THIRD_USER_ADDR.to_string()),
            game_id,
            ExpectError(4, "no settings for game id"),
        )
        .await;

    let game_id = interact
        .create_game(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            FEE_AMOUNT.into(),
            WAITING_TIME,
            2u64,
            2u64,
            WAGE_AMOUNT.into(),
        )
        .await;

    interact
        .join_game(
            &Bech32Address::from_bech32_string(THIRD_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            WAGE_AMOUNT.into(),
            game_id,
        )
        .await;

    interact
        .join_game(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            TOKEN_ID,
            0u64,
            WAGE_AMOUNT.into(),
            game_id,
        )
        .await;

    task::sleep(Duration::from_secs(100)).await;

    interact
        .send_reward(
            &Bech32Address::from_bech32_string(SECOND_USER_ADDR.to_string()),
            game_id,
        )
        .await;
}
