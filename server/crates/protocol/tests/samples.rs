//! 协议样例往返测试：遍历 `protocol/samples/v1` 下的信封样例，验证逐条可解析且
//! `type` 能判别到正确的载荷变体。

use std::path::{Path, PathBuf};

use protocol::{ClientEnvelope, ClientMessage, ProtocolError, ServerEnvelope, ServerMessage};
use serde_json::Value;

/// 样例目录：crate 根上溯三层即仓库根。
fn samples_dir(direction: &str) -> PathBuf {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../protocol/samples/v1"
    ))
    .join(direction)
}

/// 返回目录下所有 `.json` 文件，按文件名升序。
fn sample_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = std::fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("读取样例目录 {} 失败：{err}", dir.display()))
        .map(|entry| entry.expect("读取样例目录项失败").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn client_variant(message: &ClientMessage) -> &'static str {
    match message {
        ClientMessage::AuthRequest(_) => "AuthRequest",
        ClientMessage::CreateRoom(_) => "CreateRoom",
        ClientMessage::JoinRoom(_) => "JoinRoom",
        ClientMessage::LeaveRoom(_) => "LeaveRoom",
        ClientMessage::Ready(_) => "Ready",
        ClientMessage::SelectHeroCard(_) => "SelectHeroCard",
        ClientMessage::Intent(_) => "Intent",
        ClientMessage::CombatPlayed(_) => "CombatPlayed",
        ClientMessage::RequestSnapshot(_) => "RequestSnapshot",
        ClientMessage::Reconnect(_) => "Reconnect",
        ClientMessage::FriendRequest(_) => "FriendRequest",
    }
}

fn server_variant(message: &ServerMessage) -> &'static str {
    match message {
        ServerMessage::AuthResult(_) => "AuthResult",
        ServerMessage::RoomUpdate(_) => "RoomUpdate",
        ServerMessage::GameStarted(_) => "GameStarted",
        ServerMessage::Snapshot(_) => "Snapshot",
        ServerMessage::StateUpdate(_) => "StateUpdate",
        ServerMessage::PhaseChanged(_) => "PhaseChanged",
        ServerMessage::TimerWarning(_) => "TimerWarning",
        ServerMessage::CombatResult(_) => "CombatResult",
        ServerMessage::GameOver(_) => "GameOver",
        ServerMessage::Error(_) => "Error",
        ServerMessage::FriendUpdate(_) => "FriendUpdate",
    }
}

#[test]
fn c2s_samples_parse_to_matching_variant() {
    let dir = samples_dir("c2s");
    let files = sample_files(&dir);
    assert!(!files.is_empty(), "{} 下没有样例", dir.display());

    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        let expected = value["type"].as_str().unwrap().to_string();
        let envelope: ClientEnvelope = serde_json::from_value(value)
            .unwrap_or_else(|err| panic!("{} 解析信封失败：{err}", path.display()));
        let message = envelope
            .parse()
            .unwrap_or_else(|err| panic!("{} 判别失败：{err}", path.display()));
        assert_eq!(client_variant(&message), expected, "{}", path.display());
    }
}

#[test]
fn s2c_samples_parse_to_matching_variant() {
    let dir = samples_dir("s2c");
    let files = sample_files(&dir);
    assert!(!files.is_empty(), "{} 下没有样例", dir.display());

    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        let expected = value["type"].as_str().unwrap().to_string();
        let envelope: ServerEnvelope = serde_json::from_value(value)
            .unwrap_or_else(|err| panic!("{} 解析信封失败：{err}", path.display()));
        let message = envelope
            .parse()
            .unwrap_or_else(|err| panic!("{} 判别失败：{err}", path.display()));
        assert_eq!(server_variant(&message), expected, "{}", path.display());
    }
}

#[test]
fn unknown_type_is_rejected() {
    let dir = samples_dir("c2s");
    let path = sample_files(&dir).into_iter().next().unwrap();
    let mut value: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    value["type"] = Value::from("NoSuchType");
    let envelope: ClientEnvelope = serde_json::from_value(value).unwrap();
    assert_eq!(envelope.parse().unwrap_err(), ProtocolError("UNKNOWN_TYPE"));
}

#[test]
fn unknown_intent_kind_is_rejected() {
    let dir = samples_dir("c2s");
    let path = sample_files(&dir)
        .into_iter()
        .find(|path| path.file_name().is_some_and(|name| name == "Intent.json"))
        .expect("缺少 Intent.json 样例");
    let mut value: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    value["payload"]["intent"]["kind"] = Value::from("NoSuchIntent");
    let envelope: ClientEnvelope = serde_json::from_value(value).unwrap();
    assert!(envelope.parse().is_err());
}

/// 取 `s2c/StateUpdate.json` 的 payload 对象作为合法下行载荷。
fn state_update_payload() -> serde_json::Map<String, Value> {
    let path = samples_dir("s2c").join("StateUpdate.json");
    let value: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    value["payload"]
        .as_object()
        .expect("StateUpdate 样例缺少 payload 对象")
        .clone()
}

#[test]
fn server_envelope_new_and_encode_roundtrip_to_state_update() {
    let payload = state_update_payload();

    let from_new = ServerEnvelope::new("StateUpdate", 1, payload.clone()).unwrap();
    assert!(matches!(
        from_new.parse().unwrap(),
        ServerMessage::StateUpdate(_)
    ));

    let from_encode = ServerEnvelope::encode("StateUpdate", 1, &Value::Object(payload)).unwrap();
    assert!(matches!(
        from_encode.parse().unwrap(),
        ServerMessage::StateUpdate(_)
    ));
}

#[test]
fn server_envelope_new_rejects_zero_seq() {
    let payload = state_update_payload();

    assert_eq!(
        ServerEnvelope::new("StateUpdate", 0, payload.clone()).unwrap_err(),
        ProtocolError("BAD_PAYLOAD")
    );
    assert_eq!(
        ServerEnvelope::encode("StateUpdate", 0, &Value::Object(payload)).unwrap_err(),
        ProtocolError("BAD_PAYLOAD")
    );
}

#[test]
fn server_envelope_new_rejects_unknown_kind() {
    let payload = state_update_payload();

    assert_eq!(
        ServerEnvelope::new("NoSuchType", 1, payload.clone()).unwrap_err(),
        ProtocolError("UNKNOWN_TYPE")
    );
    assert_eq!(
        ServerEnvelope::encode("NoSuchType", 1, &Value::Object(payload)).unwrap_err(),
        ProtocolError("UNKNOWN_TYPE")
    );
}
