// 本文件由 `cargo run -p protocol-gen` 生成，不要手改。
/// Error types.
pub mod error {
    /// Error from a `TryFrom` or `FromStr` implementation.
    pub struct ConversionError(::std::borrow::Cow<'static, str>);
    impl ::std::error::Error for ConversionError {}
    impl ::std::fmt::Display for ConversionError {
        fn fmt(
            &self,
            f: &mut ::std::fmt::Formatter<'_>,
        ) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Display::fmt(&self.0, f)
        }
    }
    impl ::std::fmt::Debug for ConversionError {
        fn fmt(
            &self,
            f: &mut ::std::fmt::Formatter<'_>,
        ) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Debug::fmt(&self.0, f)
        }
    }
    impl From<&'static str> for ConversionError {
        fn from(value: &'static str) -> Self {
            Self(value.into())
        }
    }
    impl From<String> for ConversionError {
        fn from(value: String) -> Self {
            Self(value.into())
        }
    }
}
///`AccountView`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "displayName",
///    "highestTier",
///    "phoneBound",
///    "playerCode",
///    "playerId",
///    "rating",
///    "steamBound",
///    "tier"
///  ],
///  "properties": {
///    "displayName": {
///      "type": "string"
///    },
///    "highestTier": {
///      "description": "历史最高段位 id，按最高积分与当前配置计算",
///      "type": "string"
///    },
///    "maskedPhone": {
///      "description": "前 3 位与后 4 位保留，其余用 *；未绑定为 null",
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "phoneBound": {
///      "description": "账号是否绑定手机号",
///      "type": "boolean"
///    },
///    "playerCode": {
///      "description": "6 位玩家编号",
///      "type": "string"
///    },
///    "playerId": {
///      "description": "内部不透明标识，不向玩家展示",
///      "type": "string"
///    },
///    "rating": {
///      "description": "当前天梯积分；未结算过为 1000",
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "steamBound": {
///      "description": "账号是否绑定 Steam 身份",
///      "type": "boolean"
///    },
///    "tier": {
///      "description": "当前段位 id",
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct AccountView {
    #[serde(rename = "displayName")]
    pub display_name: ::std::string::String,
    ///历史最高段位 id，按最高积分与当前配置计算
    #[serde(rename = "highestTier")]
    pub highest_tier: ::std::string::String,
    ///前 3 位与后 4 位保留，其余用 *；未绑定为 null
    #[serde(
        rename = "maskedPhone",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub masked_phone: ::std::option::Option<::std::string::String>,
    ///账号是否绑定手机号
    #[serde(rename = "phoneBound")]
    pub phone_bound: bool,
    ///6 位玩家编号
    #[serde(rename = "playerCode")]
    pub player_code: ::std::string::String,
    ///内部不透明标识，不向玩家展示
    #[serde(rename = "playerId")]
    pub player_id: ::std::string::String,
    ///当前天梯积分；未结算过为 1000
    pub rating: u64,
    ///账号是否绑定 Steam 身份
    #[serde(rename = "steamBound")]
    pub steam_bound: bool,
    ///当前段位 id
    pub tier: ::std::string::String,
}
impl ::std::convert::From<&AccountView> for AccountView {
    fn from(value: &AccountView) -> Self {
        value.clone()
    }
}
///`AchievementState`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "unlocked"
///  ],
///  "properties": {
///    "unlocked": {
///      "type": "array",
///      "items": {
///        "type": "object",
///        "required": [
///          "achievementId",
///          "steamApiName"
///        ],
///        "properties": {
///          "achievementId": {
///            "type": "string"
///          },
///          "steamApiName": {
///            "type": "string"
///          }
///        },
///        "additionalProperties": false
///      }
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct AchievementState {
    pub unlocked: ::std::vec::Vec<AchievementStateUnlockedItem>,
}
impl ::std::convert::From<&AchievementState> for AchievementState {
    fn from(value: &AchievementState) -> Self {
        value.clone()
    }
}
///`AchievementStateUnlockedItem`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "achievementId",
///    "steamApiName"
///  ],
///  "properties": {
///    "achievementId": {
///      "type": "string"
///    },
///    "steamApiName": {
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct AchievementStateUnlockedItem {
    #[serde(rename = "achievementId")]
    pub achievement_id: ::std::string::String,
    #[serde(rename = "steamApiName")]
    pub steam_api_name: ::std::string::String,
}
impl ::std::convert::From<&AchievementStateUnlockedItem>
for AchievementStateUnlockedItem {
    fn from(value: &AchievementStateUnlockedItem) -> Self {
        value.clone()
    }
}
///`AchievementUnlocked`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "achievementId",
///    "steamApiName"
///  ],
///  "properties": {
///    "achievementId": {
///      "type": "string"
///    },
///    "steamApiName": {
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct AchievementUnlocked {
    #[serde(rename = "achievementId")]
    pub achievement_id: ::std::string::String,
    #[serde(rename = "steamApiName")]
    pub steam_api_name: ::std::string::String,
}
impl ::std::convert::From<&AchievementUnlocked> for AchievementUnlocked {
    fn from(value: &AchievementUnlocked) -> Self {
        value.clone()
    }
}
///`Affordances`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "boardMinions",
///    "canEndRecruit",
///    "canReroll",
///    "canUnfreeze",
///    "canUpgradeEra",
///    "handCards",
///    "shopSlots",
///    "upgradeDisabledReason"
///  ],
///  "properties": {
///    "boardMinions": {
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/BoardAffordance"
///      }
///    },
///    "canEndRecruit": {
///      "type": "boolean"
///    },
///    "canReroll": {
///      "type": "boolean"
///    },
///    "canUnfreeze": {
///      "description": "冻结按钮是否为「解冻」态：本回合冻结过商店且仍有冻结槽位；跨回合保留的冻结不算",
///      "type": "boolean"
///    },
///    "canUpgradeEra": {
///      "type": "boolean"
///    },
///    "handCards": {
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/HandAffordance"
///      }
///    },
///    "shopSlots": {
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/ShopAffordance"
///      }
///    },
///    "upgradeDisabledReason": {
///      "type": [
///        "string",
///        "null"
///      ]
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Affordances {
    #[serde(rename = "boardMinions")]
    pub board_minions: ::std::vec::Vec<BoardAffordance>,
    #[serde(rename = "canEndRecruit")]
    pub can_end_recruit: bool,
    #[serde(rename = "canReroll")]
    pub can_reroll: bool,
    ///冻结按钮是否为「解冻」态：本回合冻结过商店且仍有冻结槽位；跨回合保留的冻结不算
    #[serde(rename = "canUnfreeze")]
    pub can_unfreeze: bool,
    #[serde(rename = "canUpgradeEra")]
    pub can_upgrade_era: bool,
    #[serde(rename = "handCards")]
    pub hand_cards: ::std::vec::Vec<HandAffordance>,
    #[serde(rename = "shopSlots")]
    pub shop_slots: ::std::vec::Vec<ShopAffordance>,
    #[serde(rename = "upgradeDisabledReason")]
    pub upgrade_disabled_reason: ::std::option::Option<::std::string::String>,
}
impl ::std::convert::From<&Affordances> for Affordances {
    fn from(value: &Affordances) -> Self {
        value.clone()
    }
}
///`AuthRequest`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "oneOf": [
///    {
///      "type": "object",
///      "required": [
///        "code",
///        "phone"
///      ],
///      "properties": {
///        "code": {
///          "description": "短信验证码",
///          "type": "string"
///        },
///        "displayName": {
///          "description": "缺省由服务端生成「玩家N」",
///          "type": "string",
///          "maxLength": 20,
///          "minLength": 1
///        },
///        "phone": {
///          "description": "手机号",
///          "type": "string"
///        }
///      },
///      "additionalProperties": false
///    },
///    {
///      "type": "object",
///      "required": [
///        "token"
///      ],
///      "properties": {
///        "displayName": {
///          "description": "缺省由服务端生成「玩家N」",
///          "type": "string",
///          "maxLength": 20,
///          "minLength": 1
///        },
///        "token": {
///          "description": "上次登录返回的令牌",
///          "type": "string"
///        }
///      },
///      "additionalProperties": false
///    },
///    {
///      "type": "object",
///      "required": [
///        "steamTicket"
///      ],
///      "properties": {
///        "displayName": {
///          "description": "Steam 个人资料名；仅作展示，服务端截断为 20 字符",
///          "type": "string",
///          "maxLength": 64,
///          "minLength": 1
///        },
///        "steamTicket": {
///          "description": "Steam Web API 票据十六进制串",
///          "type": "string",
///          "maxLength": 2048,
///          "minLength": 1
///        }
///      },
///      "additionalProperties": false
///    }
///  ]
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(untagged, deny_unknown_fields)]
pub enum AuthRequest {
    Variant0 {
        ///短信验证码
        code: ::std::string::String,
        ///缺省由服务端生成「玩家N」
        #[serde(
            rename = "displayName",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        display_name: ::std::option::Option<AuthRequestVariant0DisplayName>,
        ///手机号
        phone: ::std::string::String,
    },
    Variant1 {
        ///缺省由服务端生成「玩家N」
        #[serde(
            rename = "displayName",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        display_name: ::std::option::Option<AuthRequestVariant1DisplayName>,
        ///上次登录返回的令牌
        token: ::std::string::String,
    },
    Variant2 {
        ///Steam 个人资料名；仅作展示，服务端截断为 20 字符
        #[serde(
            rename = "displayName",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        display_name: ::std::option::Option<AuthRequestVariant2DisplayName>,
        ///Steam Web API 票据十六进制串
        #[serde(rename = "steamTicket")]
        steam_ticket: AuthRequestVariant2SteamTicket,
    },
}
impl ::std::convert::From<&Self> for AuthRequest {
    fn from(value: &AuthRequest) -> Self {
        value.clone()
    }
}
///缺省由服务端生成「玩家N」
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "缺省由服务端生成「玩家N」",
///  "type": "string",
///  "maxLength": 20,
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct AuthRequestVariant0DisplayName(::std::string::String);
impl ::std::ops::Deref for AuthRequestVariant0DisplayName {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<AuthRequestVariant0DisplayName> for ::std::string::String {
    fn from(value: AuthRequestVariant0DisplayName) -> Self {
        value.0
    }
}
impl ::std::convert::From<&AuthRequestVariant0DisplayName>
for AuthRequestVariant0DisplayName {
    fn from(value: &AuthRequestVariant0DisplayName) -> Self {
        value.clone()
    }
}
impl ::std::str::FromStr for AuthRequestVariant0DisplayName {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() > 20usize {
            return Err("longer than 20 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for AuthRequestVariant0DisplayName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for AuthRequestVariant0DisplayName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for AuthRequestVariant0DisplayName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for AuthRequestVariant0DisplayName {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///缺省由服务端生成「玩家N」
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "缺省由服务端生成「玩家N」",
///  "type": "string",
///  "maxLength": 20,
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct AuthRequestVariant1DisplayName(::std::string::String);
impl ::std::ops::Deref for AuthRequestVariant1DisplayName {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<AuthRequestVariant1DisplayName> for ::std::string::String {
    fn from(value: AuthRequestVariant1DisplayName) -> Self {
        value.0
    }
}
impl ::std::convert::From<&AuthRequestVariant1DisplayName>
for AuthRequestVariant1DisplayName {
    fn from(value: &AuthRequestVariant1DisplayName) -> Self {
        value.clone()
    }
}
impl ::std::str::FromStr for AuthRequestVariant1DisplayName {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() > 20usize {
            return Err("longer than 20 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for AuthRequestVariant1DisplayName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for AuthRequestVariant1DisplayName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for AuthRequestVariant1DisplayName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for AuthRequestVariant1DisplayName {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///Steam 个人资料名；仅作展示，服务端截断为 20 字符
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "Steam 个人资料名；仅作展示，服务端截断为 20 字符",
///  "type": "string",
///  "maxLength": 64,
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct AuthRequestVariant2DisplayName(::std::string::String);
impl ::std::ops::Deref for AuthRequestVariant2DisplayName {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<AuthRequestVariant2DisplayName> for ::std::string::String {
    fn from(value: AuthRequestVariant2DisplayName) -> Self {
        value.0
    }
}
impl ::std::convert::From<&AuthRequestVariant2DisplayName>
for AuthRequestVariant2DisplayName {
    fn from(value: &AuthRequestVariant2DisplayName) -> Self {
        value.clone()
    }
}
impl ::std::str::FromStr for AuthRequestVariant2DisplayName {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() > 64usize {
            return Err("longer than 64 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for AuthRequestVariant2DisplayName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for AuthRequestVariant2DisplayName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for AuthRequestVariant2DisplayName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for AuthRequestVariant2DisplayName {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///Steam Web API 票据十六进制串
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "Steam Web API 票据十六进制串",
///  "type": "string",
///  "maxLength": 2048,
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct AuthRequestVariant2SteamTicket(::std::string::String);
impl ::std::ops::Deref for AuthRequestVariant2SteamTicket {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<AuthRequestVariant2SteamTicket> for ::std::string::String {
    fn from(value: AuthRequestVariant2SteamTicket) -> Self {
        value.0
    }
}
impl ::std::convert::From<&AuthRequestVariant2SteamTicket>
for AuthRequestVariant2SteamTicket {
    fn from(value: &AuthRequestVariant2SteamTicket) -> Self {
        value.clone()
    }
}
impl ::std::str::FromStr for AuthRequestVariant2SteamTicket {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() > 2048usize {
            return Err("longer than 2048 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for AuthRequestVariant2SteamTicket {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for AuthRequestVariant2SteamTicket {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for AuthRequestVariant2SteamTicket {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for AuthRequestVariant2SteamTicket {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`AuthResult`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "expiresAt",
///    "playerCode",
///    "playerId",
///    "token"
///  ],
///  "properties": {
///    "expiresAt": {
///      "type": "string",
///      "format": "date-time"
///    },
///    "playerCode": {
///      "description": "6 位玩家编号，用于好友搜索与展示",
///      "type": "string"
///    },
///    "playerId": {
///      "description": "内部不透明标识，不向玩家展示",
///      "type": "string"
///    },
///    "token": {
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct AuthResult {
    #[serde(rename = "expiresAt")]
    pub expires_at: ::chrono::DateTime<::chrono::offset::Utc>,
    ///6 位玩家编号，用于好友搜索与展示
    #[serde(rename = "playerCode")]
    pub player_code: ::std::string::String,
    ///内部不透明标识，不向玩家展示
    #[serde(rename = "playerId")]
    pub player_id: ::std::string::String,
    pub token: ::std::string::String,
}
impl ::std::convert::From<&AuthResult> for AuthResult {
    fn from(value: &AuthResult) -> Self {
        value.clone()
    }
}
///`BindPhone`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "code",
///    "phone"
///  ],
///  "properties": {
///    "code": {
///      "description": "短信验证码",
///      "type": "string"
///    },
///    "phone": {
///      "description": "待绑定的手机号",
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct BindPhone {
    ///短信验证码
    pub code: ::std::string::String,
    ///待绑定的手机号
    pub phone: ::std::string::String,
}
impl ::std::convert::From<&BindPhone> for BindPhone {
    fn from(value: &BindPhone) -> Self {
        value.clone()
    }
}
///`BoardAffordance`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "disabledReason",
///    "index",
///    "sellable"
///  ],
///  "properties": {
///    "disabledReason": {
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "index": {
///      "type": "integer",
///      "maximum": 6.0,
///      "minimum": 0.0
///    },
///    "sellable": {
///      "type": "boolean"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct BoardAffordance {
    #[serde(rename = "disabledReason")]
    pub disabled_reason: ::std::option::Option<::std::string::String>,
    pub index: i64,
    pub sellable: bool,
}
impl ::std::convert::From<&BoardAffordance> for BoardAffordance {
    fn from(value: &BoardAffordance) -> Self {
        value.clone()
    }
}
///`BoardMinion`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "attack",
///    "cardId",
///    "golden",
///    "health",
///    "index",
///    "keywords",
///    "minionId",
///    "sellable",
///    "stealth"
///  ],
///  "properties": {
///    "attack": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "cardId": {
///      "type": "string"
///    },
///    "golden": {
///      "type": "boolean"
///    },
///    "health": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "index": {
///      "description": "该随从在当前存活随从序列中的下标；上界由 PlayerView.boardSlots 决定",
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "keywords": {
///      "description": "当前生效的关键词，取值为英文 id",
///      "type": "array",
///      "items": {
///        "type": "string"
///      }
///    },
///    "minionId": {
///      "description": "随从实例 id",
///      "type": "string"
///    },
///    "sellable": {
///      "type": "boolean"
///    },
///    "stats": {
///      "description": "攻防分层构成；旧客户端可忽略",
///      "$ref": "#/$defs/MinionStats"
///    },
///    "stealth": {
///      "description": "当前是否仍处于潜行：携带 stealth 关键词且尚未被打破",
///      "type": "boolean"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct BoardMinion {
    pub attack: u64,
    #[serde(rename = "cardId")]
    pub card_id: ::std::string::String,
    pub golden: bool,
    pub health: u64,
    ///该随从在当前存活随从序列中的下标；上界由 PlayerView.boardSlots 决定
    pub index: u64,
    ///当前生效的关键词，取值为英文 id
    pub keywords: ::std::vec::Vec<::std::string::String>,
    ///随从实例 id
    #[serde(rename = "minionId")]
    pub minion_id: ::std::string::String,
    pub sellable: bool,
    ///攻防分层构成；旧客户端可忽略
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub stats: ::std::option::Option<MinionStats>,
    ///当前是否仍处于潜行：携带 stealth 关键词且尚未被打破
    pub stealth: bool,
}
impl ::std::convert::From<&BoardMinion> for BoardMinion {
    fn from(value: &BoardMinion) -> Self {
        value.clone()
    }
}
///`ClientEnvelope`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "id",
///    "payload",
///    "ts",
///    "type",
///    "v"
///  ],
///  "properties": {
///    "id": {
///      "description": "发送方生成的唯一消息 id",
///      "type": "string",
///      "minLength": 1
///    },
///    "payload": {
///      "description": "由 type 决定的结构",
///      "type": "object"
///    },
///    "playerId": {
///      "description": "认证前为 null",
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "replyTo": {
///      "description": "回复的上行消息 id",
///      "type": "string"
///    },
///    "roomId": {
///      "description": "未进房间时为 null",
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "ts": {
///      "description": "RFC 3339 UTC 时间",
///      "type": "string",
///      "format": "date-time"
///    },
///    "type": {
///      "description": "消息类型，唯一路由键",
///      "type": "string",
///      "minLength": 1
///    },
///    "v": {
///      "description": "协议主版本",
///      "const": 1
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ClientEnvelope {
    ///发送方生成的唯一消息 id
    pub id: ClientEnvelopeId,
    ///由 type 决定的结构
    pub payload: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
    ///认证前为 null
    #[serde(
        rename = "playerId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub player_id: ::std::option::Option<::std::string::String>,
    ///回复的上行消息 id
    #[serde(
        rename = "replyTo",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub reply_to: ::std::option::Option<::std::string::String>,
    ///未进房间时为 null
    #[serde(
        rename = "roomId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub room_id: ::std::option::Option<::std::string::String>,
    ///RFC 3339 UTC 时间
    pub ts: ::chrono::DateTime<::chrono::offset::Utc>,
    ///消息类型，唯一路由键
    #[serde(rename = "type")]
    pub type_: ClientEnvelopeType,
    ///协议主版本
    pub v: ::serde_json::Value,
}
impl ::std::convert::From<&ClientEnvelope> for ClientEnvelope {
    fn from(value: &ClientEnvelope) -> Self {
        value.clone()
    }
}
///发送方生成的唯一消息 id
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "发送方生成的唯一消息 id",
///  "type": "string",
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ClientEnvelopeId(::std::string::String);
impl ::std::ops::Deref for ClientEnvelopeId {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ClientEnvelopeId> for ::std::string::String {
    fn from(value: ClientEnvelopeId) -> Self {
        value.0
    }
}
impl ::std::convert::From<&ClientEnvelopeId> for ClientEnvelopeId {
    fn from(value: &ClientEnvelopeId) -> Self {
        value.clone()
    }
}
impl ::std::str::FromStr for ClientEnvelopeId {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ClientEnvelopeId {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ClientEnvelopeId {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ClientEnvelopeId {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ClientEnvelopeId {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///消息类型，唯一路由键
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "消息类型，唯一路由键",
///  "type": "string",
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ClientEnvelopeType(::std::string::String);
impl ::std::ops::Deref for ClientEnvelopeType {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ClientEnvelopeType> for ::std::string::String {
    fn from(value: ClientEnvelopeType) -> Self {
        value.0
    }
}
impl ::std::convert::From<&ClientEnvelopeType> for ClientEnvelopeType {
    fn from(value: &ClientEnvelopeType) -> Self {
        value.clone()
    }
}
impl ::std::str::FromStr for ClientEnvelopeType {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ClientEnvelopeType {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ClientEnvelopeType {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ClientEnvelopeType {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ClientEnvelopeType {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`CombatLogEntry`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "damage",
///    "ghostId",
///    "loserId",
///    "mirror",
///    "participantIds",
///    "turn",
///    "winnerId"
///  ],
///  "properties": {
///    "damage": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "ghostId": {
///      "description": "幽灵战为幽灵归属者的玩家 id；真实战为 null",
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "loserId": {
///      "description": "幽灵战失利时为该玩家；幽灵不受伤，所以幽灵输时也为 null",
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "mirror": {
///      "description": "true 表示对战已出局者的幽灵阵容",
///      "type": "boolean"
///    },
///    "participantIds": {
///      "description": "参战玩家 id；幽灵战只有对战方一个元素（幽灵不是玩家）",
///      "type": "array",
///      "items": {
///        "type": "string"
///      },
///      "maxItems": 2,
///      "minItems": 1
///    },
///    "turn": {
///      "type": "integer",
///      "maximum": 20.0,
///      "minimum": 1.0
///    },
///    "winnerId": {
///      "description": "平局无胜者时为 null",
///      "type": [
///        "string",
///        "null"
///      ]
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CombatLogEntry {
    pub damage: u64,
    ///幽灵战为幽灵归属者的玩家 id；真实战为 null
    #[serde(rename = "ghostId")]
    pub ghost_id: ::std::option::Option<::std::string::String>,
    ///幽灵战失利时为该玩家；幽灵不受伤，所以幽灵输时也为 null
    #[serde(rename = "loserId")]
    pub loser_id: ::std::option::Option<::std::string::String>,
    ///true 表示对战已出局者的幽灵阵容
    pub mirror: bool,
    ///参战玩家 id；幽灵战只有对战方一个元素（幽灵不是玩家）
    #[serde(rename = "participantIds")]
    pub participant_ids: ::std::vec::Vec<::std::string::String>,
    pub turn: ::std::num::NonZeroU64,
    ///平局无胜者时为 null
    #[serde(rename = "winnerId")]
    pub winner_id: ::std::option::Option<::std::string::String>,
}
impl ::std::convert::From<&CombatLogEntry> for CombatLogEntry {
    fn from(value: &CombatLogEntry) -> Self {
        value.clone()
    }
}
///`CombatMinionState`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "alive",
///    "attack",
///    "health",
///    "minionId"
///  ],
///  "properties": {
///    "alive": {
///      "type": "boolean"
///    },
///    "attack": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "health": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "minionId": {
///      "description": "随从实例 id",
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CombatMinionState {
    pub alive: bool,
    pub attack: u64,
    pub health: u64,
    ///随从实例 id
    #[serde(rename = "minionId")]
    pub minion_id: ::std::string::String,
}
impl ::std::convert::From<&CombatMinionState> for CombatMinionState {
    fn from(value: &CombatMinionState) -> Self {
        value.clone()
    }
}
///`CombatPlayed`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CombatPlayed {}
impl ::std::convert::From<&CombatPlayed> for CombatPlayed {
    fn from(value: &CombatPlayed) -> Self {
        value.clone()
    }
}
impl ::std::default::Default for CombatPlayed {
    fn default() -> Self {
        Self {}
    }
}
///`CombatResult`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "actions",
///    "damage",
///    "opponentBoard",
///    "ownBoard",
///    "startedAt",
///    "winnerId"
///  ],
///  "properties": {
///    "actions": {
///      "type": "array",
///      "items": {
///        "type": "object",
///        "required": [
///          "kind"
///        ],
///        "properties": {
///          "cardId": {
///            "description": "动作直接涉及的卡牌 id（如零件回收获得的牌）；没有卡牌载荷的动作省略",
///            "type": [
///              "string",
///              "null"
///            ]
///          },
///          "damage": {
///            "type": [
///              "integer",
///              "null"
///            ]
///          },
///          "kind": {
///            "description": "动作类型，具体取值由规则引擎实现确定",
///            "type": "string"
///          },
///          "source": {
///            "$ref": "#/$defs/CombatMinionState"
///          },
///          "sourceIndex": {
///            "type": [
///              "integer",
///              "null"
///            ]
///          },
///          "sourceMinionId": {
///            "description": "动作来源随从实例 id；死亡、复生、召唤等无来源动作为 null",
///            "type": [
///              "string",
///              "null"
///            ]
///          },
///          "sourceSide": {
///            "description": "sourceIndex 所指棋盘，相对接收该消息的座位；null 表示本条动作没有来源下标",
///            "type": [
///              "string",
///              "null"
///            ],
///            "enum": [
///              "own",
///              "opponent",
///              null
///            ]
///          },
///          "summon": {
///            "type": "object",
///            "required": [
///              "minion",
///              "position",
///              "side"
///            ],
///            "properties": {
///              "minion": {
///                "$ref": "#/$defs/BoardMinion"
///              },
///              "position": {
///                "description": "召唤后该随从在存活随从序列中的插入下标；上界由 PlayerView.boardSlots 决定",
///                "type": "integer",
///                "minimum": 0.0
///              },
///              "side": {
///                "description": "相对接收该消息的座位",
///                "type": "string",
///                "enum": [
///                  "own",
///                  "opponent"
///                ]
///              }
///            },
///            "additionalProperties": false
///          },
///          "target": {
///            "$ref": "#/$defs/CombatMinionState"
///          },
///          "targetIndex": {
///            "type": [
///              "integer",
///              "null"
///            ]
///          },
///          "targetMinionId": {
///            "description": "动作目标随从实例 id；死亡、复生、召唤等无目标动作为 null",
///            "type": [
///              "string",
///              "null"
///            ]
///          },
///          "targetSide": {
///            "description": "targetIndex 所指棋盘，相对接收该消息的座位；null 表示本条动作没有目标下标",
///            "type": [
///              "string",
///              "null"
///            ],
///            "enum": [
///              "own",
///              "opponent",
///              null
///            ]
///          }
///        },
///        "additionalProperties": false
///      }
///    },
///    "damage": {
///      "description": "本场战斗结算给败者的伤害（含幽灵战）；平局与幽灵战落单座位获胜时为 0",
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "mirror": {
///      "description": "true 表示本场为幽灵战，对手是已出局者的幽灵阵容",
///      "type": "boolean"
///    },
///    "opponentBoard": {
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/BoardMinion"
///      }
///    },
///    "ownBoard": {
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/BoardMinion"
///      }
///    },
///    "startedAt": {
///      "description": "本场演示开始时刻（服务端时钟，RFC3339）；重连补发时保持同值",
///      "type": "string",
///      "format": "date-time"
///    },
///    "winnerId": {
///      "description": "本场胜者；玩家自己的 playerId 表示胜",
///      "type": [
///        "string",
///        "null"
///      ]
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CombatResult {
    pub actions: ::std::vec::Vec<CombatResultActionsItem>,
    ///本场战斗结算给败者的伤害（含幽灵战）；平局与幽灵战落单座位获胜时为 0
    pub damage: u64,
    ///true 表示本场为幽灵战，对手是已出局者的幽灵阵容
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub mirror: ::std::option::Option<bool>,
    #[serde(rename = "opponentBoard")]
    pub opponent_board: ::std::vec::Vec<BoardMinion>,
    #[serde(rename = "ownBoard")]
    pub own_board: ::std::vec::Vec<BoardMinion>,
    ///本场演示开始时刻（服务端时钟，RFC3339）；重连补发时保持同值
    #[serde(rename = "startedAt")]
    pub started_at: ::chrono::DateTime<::chrono::offset::Utc>,
    ///本场胜者；玩家自己的 playerId 表示胜
    #[serde(rename = "winnerId")]
    pub winner_id: ::std::option::Option<::std::string::String>,
}
impl ::std::convert::From<&CombatResult> for CombatResult {
    fn from(value: &CombatResult) -> Self {
        value.clone()
    }
}
///`CombatResultActionsItem`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "kind"
///  ],
///  "properties": {
///    "cardId": {
///      "description": "动作直接涉及的卡牌 id（如零件回收获得的牌）；没有卡牌载荷的动作省略",
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "damage": {
///      "type": [
///        "integer",
///        "null"
///      ]
///    },
///    "kind": {
///      "description": "动作类型，具体取值由规则引擎实现确定",
///      "type": "string"
///    },
///    "source": {
///      "$ref": "#/$defs/CombatMinionState"
///    },
///    "sourceIndex": {
///      "type": [
///        "integer",
///        "null"
///      ]
///    },
///    "sourceMinionId": {
///      "description": "动作来源随从实例 id；死亡、复生、召唤等无来源动作为 null",
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "sourceSide": {
///      "description": "sourceIndex 所指棋盘，相对接收该消息的座位；null 表示本条动作没有来源下标",
///      "type": [
///        "string",
///        "null"
///      ],
///      "enum": [
///        "own",
///        "opponent",
///        null
///      ]
///    },
///    "summon": {
///      "type": "object",
///      "required": [
///        "minion",
///        "position",
///        "side"
///      ],
///      "properties": {
///        "minion": {
///          "$ref": "#/$defs/BoardMinion"
///        },
///        "position": {
///          "description": "召唤后该随从在存活随从序列中的插入下标；上界由 PlayerView.boardSlots 决定",
///          "type": "integer",
///          "minimum": 0.0
///        },
///        "side": {
///          "description": "相对接收该消息的座位",
///          "type": "string",
///          "enum": [
///            "own",
///            "opponent"
///          ]
///        }
///      },
///      "additionalProperties": false
///    },
///    "target": {
///      "$ref": "#/$defs/CombatMinionState"
///    },
///    "targetIndex": {
///      "type": [
///        "integer",
///        "null"
///      ]
///    },
///    "targetMinionId": {
///      "description": "动作目标随从实例 id；死亡、复生、召唤等无目标动作为 null",
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "targetSide": {
///      "description": "targetIndex 所指棋盘，相对接收该消息的座位；null 表示本条动作没有目标下标",
///      "type": [
///        "string",
///        "null"
///      ],
///      "enum": [
///        "own",
///        "opponent",
///        null
///      ]
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CombatResultActionsItem {
    ///动作直接涉及的卡牌 id（如零件回收获得的牌）；没有卡牌载荷的动作省略
    #[serde(
        rename = "cardId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub card_id: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub damage: ::std::option::Option<i64>,
    ///动作类型，具体取值由规则引擎实现确定
    pub kind: ::std::string::String,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub source: ::std::option::Option<CombatMinionState>,
    #[serde(
        rename = "sourceIndex",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub source_index: ::std::option::Option<i64>,
    ///动作来源随从实例 id；死亡、复生、召唤等无来源动作为 null
    #[serde(
        rename = "sourceMinionId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub source_minion_id: ::std::option::Option<::std::string::String>,
    ///sourceIndex 所指棋盘，相对接收该消息的座位；null 表示本条动作没有来源下标
    #[serde(
        rename = "sourceSide",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub source_side: ::std::option::Option<CombatResultActionsItemSourceSide>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub summon: ::std::option::Option<CombatResultActionsItemSummon>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub target: ::std::option::Option<CombatMinionState>,
    #[serde(
        rename = "targetIndex",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub target_index: ::std::option::Option<i64>,
    ///动作目标随从实例 id；死亡、复生、召唤等无目标动作为 null
    #[serde(
        rename = "targetMinionId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub target_minion_id: ::std::option::Option<::std::string::String>,
    ///targetIndex 所指棋盘，相对接收该消息的座位；null 表示本条动作没有目标下标
    #[serde(
        rename = "targetSide",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub target_side: ::std::option::Option<CombatResultActionsItemTargetSide>,
}
impl ::std::convert::From<&CombatResultActionsItem> for CombatResultActionsItem {
    fn from(value: &CombatResultActionsItem) -> Self {
        value.clone()
    }
}
///sourceIndex 所指棋盘，相对接收该消息的座位；null 表示本条动作没有来源下标
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "sourceIndex 所指棋盘，相对接收该消息的座位；null 表示本条动作没有来源下标",
///  "type": "string",
///  "enum": [
///    "own",
///    "opponent"
///  ]
///}
/// ```
/// </details>
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd
)]
pub enum CombatResultActionsItemSourceSide {
    #[serde(rename = "own")]
    Own,
    #[serde(rename = "opponent")]
    Opponent,
}
impl ::std::convert::From<&Self> for CombatResultActionsItemSourceSide {
    fn from(value: &CombatResultActionsItemSourceSide) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for CombatResultActionsItemSourceSide {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Own => f.write_str("own"),
            Self::Opponent => f.write_str("opponent"),
        }
    }
}
impl ::std::str::FromStr for CombatResultActionsItemSourceSide {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "own" => Ok(Self::Own),
            "opponent" => Ok(Self::Opponent),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for CombatResultActionsItemSourceSide {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String>
for CombatResultActionsItemSourceSide {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String>
for CombatResultActionsItemSourceSide {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///`CombatResultActionsItemSummon`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "minion",
///    "position",
///    "side"
///  ],
///  "properties": {
///    "minion": {
///      "$ref": "#/$defs/BoardMinion"
///    },
///    "position": {
///      "description": "召唤后该随从在存活随从序列中的插入下标；上界由 PlayerView.boardSlots 决定",
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "side": {
///      "description": "相对接收该消息的座位",
///      "type": "string",
///      "enum": [
///        "own",
///        "opponent"
///      ]
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CombatResultActionsItemSummon {
    pub minion: BoardMinion,
    ///召唤后该随从在存活随从序列中的插入下标；上界由 PlayerView.boardSlots 决定
    pub position: u64,
    ///相对接收该消息的座位
    pub side: CombatResultActionsItemSummonSide,
}
impl ::std::convert::From<&CombatResultActionsItemSummon>
for CombatResultActionsItemSummon {
    fn from(value: &CombatResultActionsItemSummon) -> Self {
        value.clone()
    }
}
///相对接收该消息的座位
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "相对接收该消息的座位",
///  "type": "string",
///  "enum": [
///    "own",
///    "opponent"
///  ]
///}
/// ```
/// </details>
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd
)]
pub enum CombatResultActionsItemSummonSide {
    #[serde(rename = "own")]
    Own,
    #[serde(rename = "opponent")]
    Opponent,
}
impl ::std::convert::From<&Self> for CombatResultActionsItemSummonSide {
    fn from(value: &CombatResultActionsItemSummonSide) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for CombatResultActionsItemSummonSide {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Own => f.write_str("own"),
            Self::Opponent => f.write_str("opponent"),
        }
    }
}
impl ::std::str::FromStr for CombatResultActionsItemSummonSide {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "own" => Ok(Self::Own),
            "opponent" => Ok(Self::Opponent),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for CombatResultActionsItemSummonSide {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String>
for CombatResultActionsItemSummonSide {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String>
for CombatResultActionsItemSummonSide {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///targetIndex 所指棋盘，相对接收该消息的座位；null 表示本条动作没有目标下标
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "targetIndex 所指棋盘，相对接收该消息的座位；null 表示本条动作没有目标下标",
///  "type": "string",
///  "enum": [
///    "own",
///    "opponent"
///  ]
///}
/// ```
/// </details>
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd
)]
pub enum CombatResultActionsItemTargetSide {
    #[serde(rename = "own")]
    Own,
    #[serde(rename = "opponent")]
    Opponent,
}
impl ::std::convert::From<&Self> for CombatResultActionsItemTargetSide {
    fn from(value: &CombatResultActionsItemTargetSide) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for CombatResultActionsItemTargetSide {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Own => f.write_str("own"),
            Self::Opponent => f.write_str("opponent"),
        }
    }
}
impl ::std::str::FromStr for CombatResultActionsItemTargetSide {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "own" => Ok(Self::Own),
            "opponent" => Ok(Self::Opponent),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for CombatResultActionsItemTargetSide {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String>
for CombatResultActionsItemTargetSide {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String>
for CombatResultActionsItemTargetSide {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///`CreateRoom`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CreateRoom {}
impl ::std::convert::From<&CreateRoom> for CreateRoom {
    fn from(value: &CreateRoom) -> Self {
        value.clone()
    }
}
impl ::std::default::Default for CreateRoom {
    fn default() -> Self {
        Self {}
    }
}
///`CreateRoomInvite`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CreateRoomInvite {}
impl ::std::convert::From<&CreateRoomInvite> for CreateRoomInvite {
    fn from(value: &CreateRoomInvite) -> Self {
        value.clone()
    }
}
impl ::std::default::Default for CreateRoomInvite {
    fn default() -> Self {
        Self {}
    }
}
///`DissolveRoom`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct DissolveRoom {}
impl ::std::convert::From<&DissolveRoom> for DissolveRoom {
    fn from(value: &DissolveRoom) -> Self {
        value.clone()
    }
}
impl ::std::default::Default for DissolveRoom {
    fn default() -> Self {
        Self {}
    }
}
///`Error`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "$ref": "#/$defs/ErrorMessage"
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct Error(pub ErrorMessage);
impl ::std::ops::Deref for Error {
    type Target = ErrorMessage;
    fn deref(&self) -> &ErrorMessage {
        &self.0
    }
}
impl ::std::convert::From<Error> for ErrorMessage {
    fn from(value: Error) -> Self {
        value.0
    }
}
impl ::std::convert::From<&Error> for Error {
    fn from(value: &Error) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<ErrorMessage> for Error {
    fn from(value: ErrorMessage) -> Self {
        Self(value)
    }
}
///稳定错误码，只增不改
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "稳定错误码，只增不改",
///  "type": "string",
///  "enum": [
///    "NOT_ENOUGH_GOLD",
///    "SHOP_SLOT_EMPTY",
///    "NO_SPACE",
///    "INVALID_STAGE",
///    "NO_SUCH_MINION",
///    "ALREADY_MAX_ERA",
///    "CARD_NOT_IN_HAND",
///    "HERO_NOT_SELECTED",
///    "HERO_POWER_LIMIT",
///    "SEVEN_VOYAGES_ACTIVE",
///    "NOT_ACTIVE_SKILL",
///    "INVALID_CHOICE",
///    "NO_VALID_TARGET",
///    "TARGET_NOT_ALLOWED",
///    "AUTH_FAILED",
///    "TOKEN_EXPIRED",
///    "AUTH_RATE_LIMITED",
///    "SESSION_REPLACED",
///    "BAD_PAYLOAD",
///    "UNKNOWN_TYPE",
///    "RATE_LIMITED",
///    "NOT_IN_ROOM",
///    "NOT_HOST",
///    "ROOM_NOT_FOUND",
///    "ROOM_FULL",
///    "ROOM_NOT_READY",
///    "SEQUENCE_STALE",
///    "FRIEND_NOT_FOUND",
///    "FRIEND_SELF",
///    "FRIEND_ALREADY",
///    "FRIEND_REQUEST_PENDING",
///    "FRIEND_LIST_FULL",
///    "FRIEND_REQUEST_FULL",
///    "FRIEND_REQUEST_LIMIT",
///    "NOT_FRIENDS",
///    "FRIEND_OFFLINE",
///    "FRIEND_BUSY",
///    "INVITE_NOT_FOUND",
///    "INVITE_EXPIRED",
///    "STEAM_AUTH_FAILED",
///    "STEAM_UNAVAILABLE",
///    "STEAM_DISABLED",
///    "PHONE_INVALID",
///    "PHONE_ALREADY_BOUND",
///    "PHONE_CODE_UNAVAILABLE",
///    "CODE_INVALID",
///    "CODE_EXPIRED",
///    "ALREADY_IN_ROOM",
///    "QUEUE_ALREADY",
///    "QUEUE_IN_ROOM",
///    "QUEUE_NOT_JOINED",
///    "INTERNAL_ERROR"
///  ]
///}
/// ```
/// </details>
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd
)]
pub enum ErrorCode {
    #[serde(rename = "NOT_ENOUGH_GOLD")]
    NotEnoughGold,
    #[serde(rename = "SHOP_SLOT_EMPTY")]
    ShopSlotEmpty,
    #[serde(rename = "NO_SPACE")]
    NoSpace,
    #[serde(rename = "INVALID_STAGE")]
    InvalidStage,
    #[serde(rename = "NO_SUCH_MINION")]
    NoSuchMinion,
    #[serde(rename = "ALREADY_MAX_ERA")]
    AlreadyMaxEra,
    #[serde(rename = "CARD_NOT_IN_HAND")]
    CardNotInHand,
    #[serde(rename = "HERO_NOT_SELECTED")]
    HeroNotSelected,
    #[serde(rename = "HERO_POWER_LIMIT")]
    HeroPowerLimit,
    #[serde(rename = "SEVEN_VOYAGES_ACTIVE")]
    SevenVoyagesActive,
    #[serde(rename = "NOT_ACTIVE_SKILL")]
    NotActiveSkill,
    #[serde(rename = "INVALID_CHOICE")]
    InvalidChoice,
    #[serde(rename = "NO_VALID_TARGET")]
    NoValidTarget,
    #[serde(rename = "TARGET_NOT_ALLOWED")]
    TargetNotAllowed,
    #[serde(rename = "AUTH_FAILED")]
    AuthFailed,
    #[serde(rename = "TOKEN_EXPIRED")]
    TokenExpired,
    #[serde(rename = "AUTH_RATE_LIMITED")]
    AuthRateLimited,
    #[serde(rename = "SESSION_REPLACED")]
    SessionReplaced,
    #[serde(rename = "BAD_PAYLOAD")]
    BadPayload,
    #[serde(rename = "UNKNOWN_TYPE")]
    UnknownType,
    #[serde(rename = "RATE_LIMITED")]
    RateLimited,
    #[serde(rename = "NOT_IN_ROOM")]
    NotInRoom,
    #[serde(rename = "NOT_HOST")]
    NotHost,
    #[serde(rename = "ROOM_NOT_FOUND")]
    RoomNotFound,
    #[serde(rename = "ROOM_FULL")]
    RoomFull,
    #[serde(rename = "ROOM_NOT_READY")]
    RoomNotReady,
    #[serde(rename = "SEQUENCE_STALE")]
    SequenceStale,
    #[serde(rename = "FRIEND_NOT_FOUND")]
    FriendNotFound,
    #[serde(rename = "FRIEND_SELF")]
    FriendSelf,
    #[serde(rename = "FRIEND_ALREADY")]
    FriendAlready,
    #[serde(rename = "FRIEND_REQUEST_PENDING")]
    FriendRequestPending,
    #[serde(rename = "FRIEND_LIST_FULL")]
    FriendListFull,
    #[serde(rename = "FRIEND_REQUEST_FULL")]
    FriendRequestFull,
    #[serde(rename = "FRIEND_REQUEST_LIMIT")]
    FriendRequestLimit,
    #[serde(rename = "NOT_FRIENDS")]
    NotFriends,
    #[serde(rename = "FRIEND_OFFLINE")]
    FriendOffline,
    #[serde(rename = "FRIEND_BUSY")]
    FriendBusy,
    #[serde(rename = "INVITE_NOT_FOUND")]
    InviteNotFound,
    #[serde(rename = "INVITE_EXPIRED")]
    InviteExpired,
    #[serde(rename = "STEAM_AUTH_FAILED")]
    SteamAuthFailed,
    #[serde(rename = "STEAM_UNAVAILABLE")]
    SteamUnavailable,
    #[serde(rename = "STEAM_DISABLED")]
    SteamDisabled,
    #[serde(rename = "PHONE_INVALID")]
    PhoneInvalid,
    #[serde(rename = "PHONE_ALREADY_BOUND")]
    PhoneAlreadyBound,
    #[serde(rename = "PHONE_CODE_UNAVAILABLE")]
    PhoneCodeUnavailable,
    #[serde(rename = "CODE_INVALID")]
    CodeInvalid,
    #[serde(rename = "CODE_EXPIRED")]
    CodeExpired,
    #[serde(rename = "ALREADY_IN_ROOM")]
    AlreadyInRoom,
    #[serde(rename = "QUEUE_ALREADY")]
    QueueAlready,
    #[serde(rename = "QUEUE_IN_ROOM")]
    QueueInRoom,
    #[serde(rename = "QUEUE_NOT_JOINED")]
    QueueNotJoined,
    #[serde(rename = "INTERNAL_ERROR")]
    InternalError,
}
impl ::std::convert::From<&Self> for ErrorCode {
    fn from(value: &ErrorCode) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::NotEnoughGold => f.write_str("NOT_ENOUGH_GOLD"),
            Self::ShopSlotEmpty => f.write_str("SHOP_SLOT_EMPTY"),
            Self::NoSpace => f.write_str("NO_SPACE"),
            Self::InvalidStage => f.write_str("INVALID_STAGE"),
            Self::NoSuchMinion => f.write_str("NO_SUCH_MINION"),
            Self::AlreadyMaxEra => f.write_str("ALREADY_MAX_ERA"),
            Self::CardNotInHand => f.write_str("CARD_NOT_IN_HAND"),
            Self::HeroNotSelected => f.write_str("HERO_NOT_SELECTED"),
            Self::HeroPowerLimit => f.write_str("HERO_POWER_LIMIT"),
            Self::SevenVoyagesActive => f.write_str("SEVEN_VOYAGES_ACTIVE"),
            Self::NotActiveSkill => f.write_str("NOT_ACTIVE_SKILL"),
            Self::InvalidChoice => f.write_str("INVALID_CHOICE"),
            Self::NoValidTarget => f.write_str("NO_VALID_TARGET"),
            Self::TargetNotAllowed => f.write_str("TARGET_NOT_ALLOWED"),
            Self::AuthFailed => f.write_str("AUTH_FAILED"),
            Self::TokenExpired => f.write_str("TOKEN_EXPIRED"),
            Self::AuthRateLimited => f.write_str("AUTH_RATE_LIMITED"),
            Self::SessionReplaced => f.write_str("SESSION_REPLACED"),
            Self::BadPayload => f.write_str("BAD_PAYLOAD"),
            Self::UnknownType => f.write_str("UNKNOWN_TYPE"),
            Self::RateLimited => f.write_str("RATE_LIMITED"),
            Self::NotInRoom => f.write_str("NOT_IN_ROOM"),
            Self::NotHost => f.write_str("NOT_HOST"),
            Self::RoomNotFound => f.write_str("ROOM_NOT_FOUND"),
            Self::RoomFull => f.write_str("ROOM_FULL"),
            Self::RoomNotReady => f.write_str("ROOM_NOT_READY"),
            Self::SequenceStale => f.write_str("SEQUENCE_STALE"),
            Self::FriendNotFound => f.write_str("FRIEND_NOT_FOUND"),
            Self::FriendSelf => f.write_str("FRIEND_SELF"),
            Self::FriendAlready => f.write_str("FRIEND_ALREADY"),
            Self::FriendRequestPending => f.write_str("FRIEND_REQUEST_PENDING"),
            Self::FriendListFull => f.write_str("FRIEND_LIST_FULL"),
            Self::FriendRequestFull => f.write_str("FRIEND_REQUEST_FULL"),
            Self::FriendRequestLimit => f.write_str("FRIEND_REQUEST_LIMIT"),
            Self::NotFriends => f.write_str("NOT_FRIENDS"),
            Self::FriendOffline => f.write_str("FRIEND_OFFLINE"),
            Self::FriendBusy => f.write_str("FRIEND_BUSY"),
            Self::InviteNotFound => f.write_str("INVITE_NOT_FOUND"),
            Self::InviteExpired => f.write_str("INVITE_EXPIRED"),
            Self::SteamAuthFailed => f.write_str("STEAM_AUTH_FAILED"),
            Self::SteamUnavailable => f.write_str("STEAM_UNAVAILABLE"),
            Self::SteamDisabled => f.write_str("STEAM_DISABLED"),
            Self::PhoneInvalid => f.write_str("PHONE_INVALID"),
            Self::PhoneAlreadyBound => f.write_str("PHONE_ALREADY_BOUND"),
            Self::PhoneCodeUnavailable => f.write_str("PHONE_CODE_UNAVAILABLE"),
            Self::CodeInvalid => f.write_str("CODE_INVALID"),
            Self::CodeExpired => f.write_str("CODE_EXPIRED"),
            Self::AlreadyInRoom => f.write_str("ALREADY_IN_ROOM"),
            Self::QueueAlready => f.write_str("QUEUE_ALREADY"),
            Self::QueueInRoom => f.write_str("QUEUE_IN_ROOM"),
            Self::QueueNotJoined => f.write_str("QUEUE_NOT_JOINED"),
            Self::InternalError => f.write_str("INTERNAL_ERROR"),
        }
    }
}
impl ::std::str::FromStr for ErrorCode {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "NOT_ENOUGH_GOLD" => Ok(Self::NotEnoughGold),
            "SHOP_SLOT_EMPTY" => Ok(Self::ShopSlotEmpty),
            "NO_SPACE" => Ok(Self::NoSpace),
            "INVALID_STAGE" => Ok(Self::InvalidStage),
            "NO_SUCH_MINION" => Ok(Self::NoSuchMinion),
            "ALREADY_MAX_ERA" => Ok(Self::AlreadyMaxEra),
            "CARD_NOT_IN_HAND" => Ok(Self::CardNotInHand),
            "HERO_NOT_SELECTED" => Ok(Self::HeroNotSelected),
            "HERO_POWER_LIMIT" => Ok(Self::HeroPowerLimit),
            "SEVEN_VOYAGES_ACTIVE" => Ok(Self::SevenVoyagesActive),
            "NOT_ACTIVE_SKILL" => Ok(Self::NotActiveSkill),
            "INVALID_CHOICE" => Ok(Self::InvalidChoice),
            "NO_VALID_TARGET" => Ok(Self::NoValidTarget),
            "TARGET_NOT_ALLOWED" => Ok(Self::TargetNotAllowed),
            "AUTH_FAILED" => Ok(Self::AuthFailed),
            "TOKEN_EXPIRED" => Ok(Self::TokenExpired),
            "AUTH_RATE_LIMITED" => Ok(Self::AuthRateLimited),
            "SESSION_REPLACED" => Ok(Self::SessionReplaced),
            "BAD_PAYLOAD" => Ok(Self::BadPayload),
            "UNKNOWN_TYPE" => Ok(Self::UnknownType),
            "RATE_LIMITED" => Ok(Self::RateLimited),
            "NOT_IN_ROOM" => Ok(Self::NotInRoom),
            "NOT_HOST" => Ok(Self::NotHost),
            "ROOM_NOT_FOUND" => Ok(Self::RoomNotFound),
            "ROOM_FULL" => Ok(Self::RoomFull),
            "ROOM_NOT_READY" => Ok(Self::RoomNotReady),
            "SEQUENCE_STALE" => Ok(Self::SequenceStale),
            "FRIEND_NOT_FOUND" => Ok(Self::FriendNotFound),
            "FRIEND_SELF" => Ok(Self::FriendSelf),
            "FRIEND_ALREADY" => Ok(Self::FriendAlready),
            "FRIEND_REQUEST_PENDING" => Ok(Self::FriendRequestPending),
            "FRIEND_LIST_FULL" => Ok(Self::FriendListFull),
            "FRIEND_REQUEST_FULL" => Ok(Self::FriendRequestFull),
            "FRIEND_REQUEST_LIMIT" => Ok(Self::FriendRequestLimit),
            "NOT_FRIENDS" => Ok(Self::NotFriends),
            "FRIEND_OFFLINE" => Ok(Self::FriendOffline),
            "FRIEND_BUSY" => Ok(Self::FriendBusy),
            "INVITE_NOT_FOUND" => Ok(Self::InviteNotFound),
            "INVITE_EXPIRED" => Ok(Self::InviteExpired),
            "STEAM_AUTH_FAILED" => Ok(Self::SteamAuthFailed),
            "STEAM_UNAVAILABLE" => Ok(Self::SteamUnavailable),
            "STEAM_DISABLED" => Ok(Self::SteamDisabled),
            "PHONE_INVALID" => Ok(Self::PhoneInvalid),
            "PHONE_ALREADY_BOUND" => Ok(Self::PhoneAlreadyBound),
            "PHONE_CODE_UNAVAILABLE" => Ok(Self::PhoneCodeUnavailable),
            "CODE_INVALID" => Ok(Self::CodeInvalid),
            "CODE_EXPIRED" => Ok(Self::CodeExpired),
            "ALREADY_IN_ROOM" => Ok(Self::AlreadyInRoom),
            "QUEUE_ALREADY" => Ok(Self::QueueAlready),
            "QUEUE_IN_ROOM" => Ok(Self::QueueInRoom),
            "QUEUE_NOT_JOINED" => Ok(Self::QueueNotJoined),
            "INTERNAL_ERROR" => Ok(Self::InternalError),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for ErrorCode {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ErrorCode {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ErrorCode {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///`ErrorMessage`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "code",
///    "message",
///    "messageId",
///    "retryable"
///  ],
///  "properties": {
///    "clientIntentId": {
///      "description": "被拒绝意图的 clientIntentId",
///      "type": "string"
///    },
///    "code": {
///      "description": "错误码",
///      "$ref": "#/$defs/ErrorCode"
///    },
///    "message": {
///      "description": "面向日志的错误说明，不作为 UI 文案",
///      "type": "string"
///    },
///    "messageId": {
///      "description": "面向玩家的文本 id，默认 error.<CODE>",
///      "type": "string"
///    },
///    "messageParams": {
///      "description": "messageId 的 %s 参数",
///      "type": "array",
///      "items": {
///        "type": "string"
///      }
///    },
///    "retryable": {
///      "description": "是否可原样重试",
///      "type": "boolean"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ErrorMessage {
    ///被拒绝意图的 clientIntentId
    #[serde(
        rename = "clientIntentId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub client_intent_id: ::std::option::Option<::std::string::String>,
    ///错误码
    pub code: ErrorCode,
    ///面向日志的错误说明，不作为 UI 文案
    pub message: ::std::string::String,
    ///面向玩家的文本 id，默认 error.<CODE>
    #[serde(rename = "messageId")]
    pub message_id: ::std::string::String,
    ///messageId 的 %s 参数
    #[serde(
        rename = "messageParams",
        default,
        skip_serializing_if = "::std::vec::Vec::is_empty"
    )]
    pub message_params: ::std::vec::Vec<::std::string::String>,
    ///是否可原样重试
    pub retryable: bool,
}
impl ::std::convert::From<&ErrorMessage> for ErrorMessage {
    fn from(value: &ErrorMessage) -> Self {
        value.clone()
    }
}
///`FriendInvite`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "expiresAt",
///    "fromDisplayName",
///    "fromPlayerCode",
///    "fromPlayerId",
///    "inviteId",
///    "roomCode"
///  ],
///  "properties": {
///    "expiresAt": {
///      "type": "string",
///      "format": "date-time"
///    },
///    "fromDisplayName": {
///      "type": "string"
///    },
///    "fromPlayerCode": {
///      "type": "string"
///    },
///    "fromPlayerId": {
///      "type": "string"
///    },
///    "inviteId": {
///      "type": "string"
///    },
///    "roomCode": {
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct FriendInvite {
    #[serde(rename = "expiresAt")]
    pub expires_at: ::chrono::DateTime<::chrono::offset::Utc>,
    #[serde(rename = "fromDisplayName")]
    pub from_display_name: ::std::string::String,
    #[serde(rename = "fromPlayerCode")]
    pub from_player_code: ::std::string::String,
    #[serde(rename = "fromPlayerId")]
    pub from_player_id: ::std::string::String,
    #[serde(rename = "inviteId")]
    pub invite_id: ::std::string::String,
    #[serde(rename = "roomCode")]
    pub room_code: ::std::string::String,
}
impl ::std::convert::From<&FriendInvite> for FriendInvite {
    fn from(value: &FriendInvite) -> Self {
        value.clone()
    }
}
///`FriendInviteResult`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "accepted",
///    "inviteId"
///  ],
///  "properties": {
///    "accepted": {
///      "type": "boolean"
///    },
///    "inviteId": {
///      "type": "string"
///    },
///    "reason": {
///      "description": "拒绝或失败原因的错误码",
///      "type": [
///        "string",
///        "null"
///      ]
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct FriendInviteResult {
    pub accepted: bool,
    #[serde(rename = "inviteId")]
    pub invite_id: ::std::string::String,
    ///拒绝或失败原因的错误码
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub reason: ::std::option::Option<::std::string::String>,
}
impl ::std::convert::From<&FriendInviteResult> for FriendInviteResult {
    fn from(value: &FriendInviteResult) -> Self {
        value.clone()
    }
}
///`FriendRemove`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "friendId"
///  ],
///  "properties": {
///    "friendId": {
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct FriendRemove {
    #[serde(rename = "friendId")]
    pub friend_id: ::std::string::String,
}
impl ::std::convert::From<&FriendRemove> for FriendRemove {
    fn from(value: &FriendRemove) -> Self {
        value.clone()
    }
}
///`FriendRequest`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "toPlayerCode"
///  ],
///  "properties": {
///    "toPlayerCode": {
///      "description": "6 位玩家编号",
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct FriendRequest {
    ///6 位玩家编号
    #[serde(rename = "toPlayerCode")]
    pub to_player_code: ::std::string::String,
}
impl ::std::convert::From<&FriendRequest> for FriendRequest {
    fn from(value: &FriendRequest) -> Self {
        value.clone()
    }
}
///`FriendRequestItem`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "displayName",
///    "playerCode",
///    "playerId",
///    "sentAt"
///  ],
///  "properties": {
///    "displayName": {
///      "type": "string"
///    },
///    "playerCode": {
///      "type": "string"
///    },
///    "playerId": {
///      "type": "string"
///    },
///    "sentAt": {
///      "type": "string",
///      "format": "date-time"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct FriendRequestItem {
    #[serde(rename = "displayName")]
    pub display_name: ::std::string::String,
    #[serde(rename = "playerCode")]
    pub player_code: ::std::string::String,
    #[serde(rename = "playerId")]
    pub player_id: ::std::string::String,
    #[serde(rename = "sentAt")]
    pub sent_at: ::chrono::DateTime<::chrono::offset::Utc>,
}
impl ::std::convert::From<&FriendRequestItem> for FriendRequestItem {
    fn from(value: &FriendRequestItem) -> Self {
        value.clone()
    }
}
///`FriendRespond`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "accept",
///    "fromPlayerId"
///  ],
///  "properties": {
///    "accept": {
///      "type": "boolean"
///    },
///    "fromPlayerId": {
///      "description": "申请人的 playerId",
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct FriendRespond {
    pub accept: bool,
    ///申请人的 playerId
    #[serde(rename = "fromPlayerId")]
    pub from_player_id: ::std::string::String,
}
impl ::std::convert::From<&FriendRespond> for FriendRespond {
    fn from(value: &FriendRespond) -> Self {
        value.clone()
    }
}
///`FriendUpdate`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "friends",
///    "incoming",
///    "outgoing",
///    "recent"
///  ],
///  "properties": {
///    "friends": {
///      "type": "array",
///      "items": {
///        "type": "object",
///        "required": [
///          "displayName",
///          "playerCode",
///          "playerId",
///          "status"
///        ],
///        "properties": {
///          "displayName": {
///            "type": "string"
///          },
///          "playerCode": {
///            "description": "6 位玩家编号",
///            "type": "string"
///          },
///          "playerId": {
///            "type": "string"
///          },
///          "status": {
///            "type": "string",
///            "enum": [
///              "online",
///              "inGame",
///              "offline"
///            ]
///          }
///        },
///        "additionalProperties": false
///      }
///    },
///    "incoming": {
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/FriendRequestItem"
///      }
///    },
///    "outgoing": {
///      "type": "array",
///      "items": {
///        "type": "object",
///        "required": [
///          "displayName",
///          "playerCode",
///          "playerId",
///          "sentAt"
///        ],
///        "properties": {
///          "displayName": {
///            "type": "string"
///          },
///          "playerCode": {
///            "type": "string"
///          },
///          "playerId": {
///            "type": "string"
///          },
///          "sentAt": {
///            "type": "string",
///            "format": "date-time"
///          }
///        },
///        "additionalProperties": false
///      }
///    },
///    "recent": {
///      "type": "array",
///      "items": {
///        "type": "object",
///        "required": [
///          "displayName",
///          "playerCode",
///          "playerId",
///          "relation"
///        ],
///        "properties": {
///          "displayName": {
///            "type": "string"
///          },
///          "playerCode": {
///            "type": "string"
///          },
///          "playerId": {
///            "type": "string"
///          },
///          "relation": {
///            "type": "string",
///            "enum": [
///              "none",
///              "friend",
///              "incoming",
///              "outgoing"
///            ]
///          }
///        },
///        "additionalProperties": false
///      }
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct FriendUpdate {
    pub friends: ::std::vec::Vec<FriendUpdateFriendsItem>,
    pub incoming: ::std::vec::Vec<FriendRequestItem>,
    pub outgoing: ::std::vec::Vec<FriendUpdateOutgoingItem>,
    pub recent: ::std::vec::Vec<FriendUpdateRecentItem>,
}
impl ::std::convert::From<&FriendUpdate> for FriendUpdate {
    fn from(value: &FriendUpdate) -> Self {
        value.clone()
    }
}
///`FriendUpdateFriendsItem`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "displayName",
///    "playerCode",
///    "playerId",
///    "status"
///  ],
///  "properties": {
///    "displayName": {
///      "type": "string"
///    },
///    "playerCode": {
///      "description": "6 位玩家编号",
///      "type": "string"
///    },
///    "playerId": {
///      "type": "string"
///    },
///    "status": {
///      "type": "string",
///      "enum": [
///        "online",
///        "inGame",
///        "offline"
///      ]
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct FriendUpdateFriendsItem {
    #[serde(rename = "displayName")]
    pub display_name: ::std::string::String,
    ///6 位玩家编号
    #[serde(rename = "playerCode")]
    pub player_code: ::std::string::String,
    #[serde(rename = "playerId")]
    pub player_id: ::std::string::String,
    pub status: FriendUpdateFriendsItemStatus,
}
impl ::std::convert::From<&FriendUpdateFriendsItem> for FriendUpdateFriendsItem {
    fn from(value: &FriendUpdateFriendsItem) -> Self {
        value.clone()
    }
}
///`FriendUpdateFriendsItemStatus`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "enum": [
///    "online",
///    "inGame",
///    "offline"
///  ]
///}
/// ```
/// </details>
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd
)]
pub enum FriendUpdateFriendsItemStatus {
    #[serde(rename = "online")]
    Online,
    #[serde(rename = "inGame")]
    InGame,
    #[serde(rename = "offline")]
    Offline,
}
impl ::std::convert::From<&Self> for FriendUpdateFriendsItemStatus {
    fn from(value: &FriendUpdateFriendsItemStatus) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for FriendUpdateFriendsItemStatus {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Online => f.write_str("online"),
            Self::InGame => f.write_str("inGame"),
            Self::Offline => f.write_str("offline"),
        }
    }
}
impl ::std::str::FromStr for FriendUpdateFriendsItemStatus {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "online" => Ok(Self::Online),
            "inGame" => Ok(Self::InGame),
            "offline" => Ok(Self::Offline),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for FriendUpdateFriendsItemStatus {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for FriendUpdateFriendsItemStatus {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for FriendUpdateFriendsItemStatus {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///`FriendUpdateOutgoingItem`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "displayName",
///    "playerCode",
///    "playerId",
///    "sentAt"
///  ],
///  "properties": {
///    "displayName": {
///      "type": "string"
///    },
///    "playerCode": {
///      "type": "string"
///    },
///    "playerId": {
///      "type": "string"
///    },
///    "sentAt": {
///      "type": "string",
///      "format": "date-time"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct FriendUpdateOutgoingItem {
    #[serde(rename = "displayName")]
    pub display_name: ::std::string::String,
    #[serde(rename = "playerCode")]
    pub player_code: ::std::string::String,
    #[serde(rename = "playerId")]
    pub player_id: ::std::string::String,
    #[serde(rename = "sentAt")]
    pub sent_at: ::chrono::DateTime<::chrono::offset::Utc>,
}
impl ::std::convert::From<&FriendUpdateOutgoingItem> for FriendUpdateOutgoingItem {
    fn from(value: &FriendUpdateOutgoingItem) -> Self {
        value.clone()
    }
}
///`FriendUpdateRecentItem`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "displayName",
///    "playerCode",
///    "playerId",
///    "relation"
///  ],
///  "properties": {
///    "displayName": {
///      "type": "string"
///    },
///    "playerCode": {
///      "type": "string"
///    },
///    "playerId": {
///      "type": "string"
///    },
///    "relation": {
///      "type": "string",
///      "enum": [
///        "none",
///        "friend",
///        "incoming",
///        "outgoing"
///      ]
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct FriendUpdateRecentItem {
    #[serde(rename = "displayName")]
    pub display_name: ::std::string::String,
    #[serde(rename = "playerCode")]
    pub player_code: ::std::string::String,
    #[serde(rename = "playerId")]
    pub player_id: ::std::string::String,
    pub relation: FriendUpdateRecentItemRelation,
}
impl ::std::convert::From<&FriendUpdateRecentItem> for FriendUpdateRecentItem {
    fn from(value: &FriendUpdateRecentItem) -> Self {
        value.clone()
    }
}
///`FriendUpdateRecentItemRelation`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "enum": [
///    "none",
///    "friend",
///    "incoming",
///    "outgoing"
///  ]
///}
/// ```
/// </details>
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd
)]
pub enum FriendUpdateRecentItemRelation {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "friend")]
    Friend,
    #[serde(rename = "incoming")]
    Incoming,
    #[serde(rename = "outgoing")]
    Outgoing,
}
impl ::std::convert::From<&Self> for FriendUpdateRecentItemRelation {
    fn from(value: &FriendUpdateRecentItemRelation) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for FriendUpdateRecentItemRelation {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::None => f.write_str("none"),
            Self::Friend => f.write_str("friend"),
            Self::Incoming => f.write_str("incoming"),
            Self::Outgoing => f.write_str("outgoing"),
        }
    }
}
impl ::std::str::FromStr for FriendUpdateRecentItemRelation {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "none" => Ok(Self::None),
            "friend" => Ok(Self::Friend),
            "incoming" => Ok(Self::Incoming),
            "outgoing" => Ok(Self::Outgoing),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for FriendUpdateRecentItemRelation {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for FriendUpdateRecentItemRelation {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for FriendUpdateRecentItemRelation {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///`GameEvent`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "kind"
///  ],
///  "properties": {
///    "amount": {
///      "description": "伤害等数值",
///      "type": "integer"
///    },
///    "cardId": {
///      "type": "string"
///    },
///    "kind": {
///      "type": "string",
///      "enum": [
///        "MinionBought",
///        "MinionSold",
///        "ShopRolled",
///        "EraUpgraded",
///        "TripleFormed",
///        "CardDiscovered",
///        "DamageTaken",
///        "PlayerEliminated",
///        "TurnStarted",
///        "PhaseChanged",
///        "GameOver"
///      ]
///    },
///    "minionId": {
///      "type": "string"
///    },
///    "phase": {
///      "description": "PhaseChanged 事件进入的阶段",
///      "$ref": "#/$defs/Phase"
///    },
///    "place": {
///      "description": "PlayerEliminated 事件对该座位判定的名次；其他事件为 null",
///      "type": [
///        "integer",
///        "null"
///      ],
///      "maximum": 8.0,
///      "minimum": 1.0
///    },
///    "playerId": {
///      "description": "事件涉及的玩家，可省略表示自己",
///      "type": "string"
///    },
///    "ranking": {
///      "description": "GameOver 事件的座位排名，从第 1 名起",
///      "type": "array",
///      "items": {
///        "type": "integer",
///        "maximum": 7.0,
///        "minimum": 0.0
///      }
///    },
///    "seat": {
///      "description": "事件涉及的座位；客户端可经 PlayerView.players 映射到 playerId",
///      "type": "integer",
///      "maximum": 7.0,
///      "minimum": 0.0
///    },
///    "turn": {
///      "description": "TurnStarted/PhaseChanged 事件对应的回合号",
///      "type": "integer",
///      "maximum": 20.0,
///      "minimum": 1.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct GameEvent {
    ///伤害等数值
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub amount: ::std::option::Option<i64>,
    #[serde(
        rename = "cardId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub card_id: ::std::option::Option<::std::string::String>,
    pub kind: GameEventKind,
    #[serde(
        rename = "minionId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub minion_id: ::std::option::Option<::std::string::String>,
    ///PhaseChanged 事件进入的阶段
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub phase: ::std::option::Option<Phase>,
    ///PlayerEliminated 事件对该座位判定的名次；其他事件为 null
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub place: ::std::option::Option<::std::num::NonZeroU64>,
    ///事件涉及的玩家，可省略表示自己
    #[serde(
        rename = "playerId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub player_id: ::std::option::Option<::std::string::String>,
    ///GameOver 事件的座位排名，从第 1 名起
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub ranking: ::std::vec::Vec<i64>,
    ///事件涉及的座位；客户端可经 PlayerView.players 映射到 playerId
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub seat: ::std::option::Option<i64>,
    ///TurnStarted/PhaseChanged 事件对应的回合号
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub turn: ::std::option::Option<::std::num::NonZeroU64>,
}
impl ::std::convert::From<&GameEvent> for GameEvent {
    fn from(value: &GameEvent) -> Self {
        value.clone()
    }
}
///`GameEventKind`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "enum": [
///    "MinionBought",
///    "MinionSold",
///    "ShopRolled",
///    "EraUpgraded",
///    "TripleFormed",
///    "CardDiscovered",
///    "DamageTaken",
///    "PlayerEliminated",
///    "TurnStarted",
///    "PhaseChanged",
///    "GameOver"
///  ]
///}
/// ```
/// </details>
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd
)]
pub enum GameEventKind {
    MinionBought,
    MinionSold,
    ShopRolled,
    EraUpgraded,
    TripleFormed,
    CardDiscovered,
    DamageTaken,
    PlayerEliminated,
    TurnStarted,
    PhaseChanged,
    GameOver,
}
impl ::std::convert::From<&Self> for GameEventKind {
    fn from(value: &GameEventKind) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for GameEventKind {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::MinionBought => f.write_str("MinionBought"),
            Self::MinionSold => f.write_str("MinionSold"),
            Self::ShopRolled => f.write_str("ShopRolled"),
            Self::EraUpgraded => f.write_str("EraUpgraded"),
            Self::TripleFormed => f.write_str("TripleFormed"),
            Self::CardDiscovered => f.write_str("CardDiscovered"),
            Self::DamageTaken => f.write_str("DamageTaken"),
            Self::PlayerEliminated => f.write_str("PlayerEliminated"),
            Self::TurnStarted => f.write_str("TurnStarted"),
            Self::PhaseChanged => f.write_str("PhaseChanged"),
            Self::GameOver => f.write_str("GameOver"),
        }
    }
}
impl ::std::str::FromStr for GameEventKind {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "MinionBought" => Ok(Self::MinionBought),
            "MinionSold" => Ok(Self::MinionSold),
            "ShopRolled" => Ok(Self::ShopRolled),
            "EraUpgraded" => Ok(Self::EraUpgraded),
            "TripleFormed" => Ok(Self::TripleFormed),
            "CardDiscovered" => Ok(Self::CardDiscovered),
            "DamageTaken" => Ok(Self::DamageTaken),
            "PlayerEliminated" => Ok(Self::PlayerEliminated),
            "TurnStarted" => Ok(Self::TurnStarted),
            "PhaseChanged" => Ok(Self::PhaseChanged),
            "GameOver" => Ok(Self::GameOver),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for GameEventKind {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for GameEventKind {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for GameEventKind {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///`GameOver`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "ranking",
///    "ratingAfter",
///    "ratingDelta",
///    "seed",
///    "tierAfter",
///    "tierBefore"
///  ],
///  "properties": {
///    "ranking": {
///      "type": "array",
///      "items": {
///        "type": "object",
///        "required": [
///          "place",
///          "playerId"
///        ],
///        "properties": {
///          "damageDealt": {
///            "description": "该座位本局造成的总伤害；旧客户端缺省按 0 处理",
///            "type": "integer",
///            "minimum": 0.0
///          },
///          "place": {
///            "type": "integer",
///            "maximum": 8.0,
///            "minimum": 1.0
///          },
///          "playerId": {
///            "type": "string"
///          },
///          "survivedTurns": {
///            "description": "该座位存活到的回合；旧客户端缺省按 0 处理",
///            "type": "integer",
///            "maximum": 20.0,
///            "minimum": 0.0
///          }
///        },
///        "additionalProperties": false
///      }
///    },
///    "ratingAfter": {
///      "description": "结算后个人总分；房间局为 null",
///      "type": [
///        "integer",
///        "null"
///      ],
///      "minimum": 0.0
///    },
///    "ratingDelta": {
///      "description": "本局实际入账积分变化；房间局为 null",
///      "type": [
///        "integer",
///        "null"
///      ]
///    },
///    "seed": {
///      "description": "会战模式对局结束后公开的种子",
///      "type": "string"
///    },
///    "selfStats": {
///      "description": "接收者本人的本局统计；只填接收者所在座位",
///      "$ref": "#/$defs/SelfStats"
///    },
///    "tierAfter": {
///      "description": "结算后段位 id；房间局为 null",
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "tierBefore": {
///      "description": "结算前段位 id；房间局为 null",
///      "type": [
///        "string",
///        "null"
///      ]
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct GameOver {
    pub ranking: ::std::vec::Vec<GameOverRankingItem>,
    ///结算后个人总分；房间局为 null
    #[serde(rename = "ratingAfter")]
    pub rating_after: ::std::option::Option<u64>,
    ///本局实际入账积分变化；房间局为 null
    #[serde(rename = "ratingDelta")]
    pub rating_delta: ::std::option::Option<i64>,
    ///会战模式对局结束后公开的种子
    pub seed: ::std::string::String,
    ///接收者本人的本局统计；只填接收者所在座位
    #[serde(
        rename = "selfStats",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub self_stats: ::std::option::Option<SelfStats>,
    ///结算后段位 id；房间局为 null
    #[serde(rename = "tierAfter")]
    pub tier_after: ::std::option::Option<::std::string::String>,
    ///结算前段位 id；房间局为 null
    #[serde(rename = "tierBefore")]
    pub tier_before: ::std::option::Option<::std::string::String>,
}
impl ::std::convert::From<&GameOver> for GameOver {
    fn from(value: &GameOver) -> Self {
        value.clone()
    }
}
///`GameOverRankingItem`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "place",
///    "playerId"
///  ],
///  "properties": {
///    "damageDealt": {
///      "description": "该座位本局造成的总伤害；旧客户端缺省按 0 处理",
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "place": {
///      "type": "integer",
///      "maximum": 8.0,
///      "minimum": 1.0
///    },
///    "playerId": {
///      "type": "string"
///    },
///    "survivedTurns": {
///      "description": "该座位存活到的回合；旧客户端缺省按 0 处理",
///      "type": "integer",
///      "maximum": 20.0,
///      "minimum": 0.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct GameOverRankingItem {
    ///该座位本局造成的总伤害；旧客户端缺省按 0 处理
    #[serde(
        rename = "damageDealt",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub damage_dealt: ::std::option::Option<u64>,
    pub place: ::std::num::NonZeroU64,
    #[serde(rename = "playerId")]
    pub player_id: ::std::string::String,
    ///该座位存活到的回合；旧客户端缺省按 0 处理
    #[serde(
        rename = "survivedTurns",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub survived_turns: ::std::option::Option<i64>,
}
impl ::std::convert::From<&GameOverRankingItem> for GameOverRankingItem {
    fn from(value: &GameOverRankingItem) -> Self {
        value.clone()
    }
}
///`GameStarted`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "view"
///  ],
///  "properties": {
///    "view": {
///      "$ref": "#/$defs/PlayerView"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct GameStarted {
    pub view: PlayerView,
}
impl ::std::convert::From<&GameStarted> for GameStarted {
    fn from(value: &GameStarted) -> Self {
        value.clone()
    }
}
///`HandAffordance`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "disabledReason",
///    "index",
///    "playable"
///  ],
///  "properties": {
///    "disabledReason": {
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "index": {
///      "type": "integer",
///      "maximum": 9.0,
///      "minimum": 0.0
///    },
///    "playable": {
///      "type": "boolean"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct HandAffordance {
    #[serde(rename = "disabledReason")]
    pub disabled_reason: ::std::option::Option<::std::string::String>,
    pub index: i64,
    pub playable: bool,
}
impl ::std::convert::From<&HandAffordance> for HandAffordance {
    fn from(value: &HandAffordance) -> Self {
        value.clone()
    }
}
///`HandCard`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "attack",
///    "cardId",
///    "disabledReason",
///    "golden",
///    "health",
///    "index",
///    "instanceId",
///    "playable",
///    "requiresTarget",
///    "rewardLevel",
///    "targetCandidates"
///  ],
///  "properties": {
///    "attack": {
///      "description": "随从牌的当前攻击力（已含成长等全部层数）；效果牌为 null",
///      "type": [
///        "integer",
///        "null"
///      ]
///    },
///    "cardId": {
///      "type": "string"
///    },
///    "disabledReason": {
///      "description": "不可操作时的错误码",
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "golden": {
///      "description": "随从牌为金色三连结果；效果牌恒为 false",
///      "type": "boolean"
///    },
///    "health": {
///      "description": "随从牌的当前生命值；效果牌为 null",
///      "type": [
///        "integer",
///        "null"
///      ]
///    },
///    "index": {
///      "type": "integer",
///      "maximum": 9.0,
///      "minimum": 0.0
///    },
///    "instanceId": {
///      "description": "卡牌实例编号；买入、上手与三连合成沿用同一编号",
///      "type": "string"
///    },
///    "playable": {
///      "description": "服务端算好的可打出提示",
///      "type": "boolean"
///    },
///    "requiresTarget": {
///      "description": "该牌是否需要玩家指定单一目标",
///      "type": "boolean"
///    },
///    "rewardLevel": {
///      "description": "「三连奖励」锁定的发现星级；其它牌为 null",
///      "type": [
///        "integer",
///        "null"
///      ],
///      "maximum": 6.0,
///      "minimum": 1.0
///    },
///    "targetCandidates": {
///      "description": "该牌的合法目标；随从牌为空数组",
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/TargetRef"
///      }
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct HandCard {
    ///随从牌的当前攻击力（已含成长等全部层数）；效果牌为 null
    pub attack: ::std::option::Option<i64>,
    #[serde(rename = "cardId")]
    pub card_id: ::std::string::String,
    ///不可操作时的错误码
    #[serde(rename = "disabledReason")]
    pub disabled_reason: ::std::option::Option<::std::string::String>,
    ///随从牌为金色三连结果；效果牌恒为 false
    pub golden: bool,
    ///随从牌的当前生命值；效果牌为 null
    pub health: ::std::option::Option<i64>,
    pub index: i64,
    ///卡牌实例编号；买入、上手与三连合成沿用同一编号
    #[serde(rename = "instanceId")]
    pub instance_id: ::std::string::String,
    ///服务端算好的可打出提示
    pub playable: bool,
    ///该牌是否需要玩家指定单一目标
    #[serde(rename = "requiresTarget")]
    pub requires_target: bool,
    ///「三连奖励」锁定的发现星级；其它牌为 null
    #[serde(rename = "rewardLevel")]
    pub reward_level: ::std::option::Option<::std::num::NonZeroU64>,
    ///该牌的合法目标；随从牌为空数组
    #[serde(rename = "targetCandidates")]
    pub target_candidates: ::std::vec::Vec<TargetRef>,
}
impl ::std::convert::From<&HandCard> for HandCard {
    fn from(value: &HandCard) -> Self {
        value.clone()
    }
}
///`HeroPickStarted`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "candidates",
///    "deadline",
///    "players"
///  ],
///  "properties": {
///    "candidates": {
///      "description": "收件人本座位可选的英雄卡 id；该消息按座位逐条下发",
///      "type": "array",
///      "items": {
///        "type": "string"
///      }
///    },
///    "deadline": {
///      "description": "选英雄截止时刻（RFC3339）",
///      "type": "string"
///    },
///    "players": {
///      "type": "array",
///      "items": {
///        "type": "object",
///        "required": [
///          "displayName",
///          "isBot",
///          "picked",
///          "playerId",
///          "seat"
///        ],
///        "properties": {
///          "displayName": {
///            "type": "string"
///          },
///          "isBot": {
///            "type": "boolean"
///          },
///          "picked": {
///            "description": "该座位是否已确认英雄",
///            "type": "boolean"
///          },
///          "playerId": {
///            "description": "该座位玩家 id；客户端据此识别本人座位",
///            "type": "string"
///          },
///          "seat": {
///            "type": "integer",
///            "maximum": 7.0,
///            "minimum": 0.0
///          }
///        },
///        "additionalProperties": false
///      }
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct HeroPickStarted {
    ///收件人本座位可选的英雄卡 id；该消息按座位逐条下发
    pub candidates: ::std::vec::Vec<::std::string::String>,
    ///选英雄截止时刻（RFC3339）
    pub deadline: ::std::string::String,
    pub players: ::std::vec::Vec<HeroPickStartedPlayersItem>,
}
impl ::std::convert::From<&HeroPickStarted> for HeroPickStarted {
    fn from(value: &HeroPickStarted) -> Self {
        value.clone()
    }
}
///`HeroPickStartedPlayersItem`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "displayName",
///    "isBot",
///    "picked",
///    "playerId",
///    "seat"
///  ],
///  "properties": {
///    "displayName": {
///      "type": "string"
///    },
///    "isBot": {
///      "type": "boolean"
///    },
///    "picked": {
///      "description": "该座位是否已确认英雄",
///      "type": "boolean"
///    },
///    "playerId": {
///      "description": "该座位玩家 id；客户端据此识别本人座位",
///      "type": "string"
///    },
///    "seat": {
///      "type": "integer",
///      "maximum": 7.0,
///      "minimum": 0.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct HeroPickStartedPlayersItem {
    #[serde(rename = "displayName")]
    pub display_name: ::std::string::String,
    #[serde(rename = "isBot")]
    pub is_bot: bool,
    ///该座位是否已确认英雄
    pub picked: bool,
    ///该座位玩家 id；客户端据此识别本人座位
    #[serde(rename = "playerId")]
    pub player_id: ::std::string::String,
    pub seat: i64,
}
impl ::std::convert::From<&HeroPickStartedPlayersItem> for HeroPickStartedPlayersItem {
    fn from(value: &HeroPickStartedPlayersItem) -> Self {
        value.clone()
    }
}
///`HeroPower`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "canUse",
///    "disabledReason",
///    "requiresTarget",
///    "skillId",
///    "targetCandidates",
///    "usesLeft",
///    "voyagePending",
///    "voyageRemaining"
///  ],
///  "properties": {
///    "canUse": {
///      "type": "boolean"
///    },
///    "disabledReason": {
///      "description": "不可使用该技能时的错误码",
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "requiresTarget": {
///      "description": "该主动技能是否需要玩家指定单一目标",
///      "type": "boolean"
///    },
///    "skillId": {
///      "type": "string"
///    },
///    "targetCandidates": {
///      "description": "该主动技能的合法目标",
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/TargetRef"
///      }
///    },
///    "usesLeft": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "voyagePending": {
///      "description": "七下西洋已累计、未到账的金币；没有航程时为 0",
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "voyageRemaining": {
///      "description": "七下西洋剩余航行回合；没有航程时为 null",
///      "type": [
///        "integer",
///        "null"
///      ],
///      "maximum": 7.0,
///      "minimum": 0.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct HeroPower {
    #[serde(rename = "canUse")]
    pub can_use: bool,
    ///不可使用该技能时的错误码
    #[serde(rename = "disabledReason")]
    pub disabled_reason: ::std::option::Option<::std::string::String>,
    ///该主动技能是否需要玩家指定单一目标
    #[serde(rename = "requiresTarget")]
    pub requires_target: bool,
    #[serde(rename = "skillId")]
    pub skill_id: ::std::string::String,
    ///该主动技能的合法目标
    #[serde(rename = "targetCandidates")]
    pub target_candidates: ::std::vec::Vec<TargetRef>,
    #[serde(rename = "usesLeft")]
    pub uses_left: u64,
    ///七下西洋已累计、未到账的金币；没有航程时为 0
    #[serde(rename = "voyagePending")]
    pub voyage_pending: u64,
    ///七下西洋剩余航行回合；没有航程时为 null
    #[serde(rename = "voyageRemaining")]
    pub voyage_remaining: ::std::option::Option<i64>,
}
impl ::std::convert::From<&HeroPower> for HeroPower {
    fn from(value: &HeroPower) -> Self {
        value.clone()
    }
}
///`Intent`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "oneOf": [
///    {
///      "type": "object",
///      "required": [
///        "kind",
///        "slot"
///      ],
///      "properties": {
///        "kind": {
///          "const": "BuyMinion"
///        },
///        "slot": {
///          "description": "商店槽位",
///          "type": "integer",
///          "maximum": 5.0,
///          "minimum": 0.0
///        }
///      },
///      "additionalProperties": false
///    },
///    {
///      "type": "object",
///      "required": [
///        "kind",
///        "minionId"
///      ],
///      "properties": {
///        "kind": {
///          "const": "SellMinion"
///        },
///        "minionId": {
///          "description": "要出售的随从实例 id",
///          "type": "string"
///        }
///      },
///      "additionalProperties": false
///    },
///    {
///      "type": "object",
///      "required": [
///        "kind"
///      ],
///      "properties": {
///        "kind": {
///          "const": "RerollShop"
///        }
///      },
///      "additionalProperties": false
///    },
///    {
///      "type": "object",
///      "required": [
///        "kind"
///      ],
///      "properties": {
///        "kind": {
///          "const": "UpgradeEra"
///        }
///      },
///      "additionalProperties": false
///    },
///    {
///      "type": "object",
///      "required": [
///        "handIndex",
///        "kind"
///      ],
///      "properties": {
///        "handIndex": {
///          "description": "手牌位置",
///          "type": "integer",
///          "maximum": 9.0,
///          "minimum": 0.0
///        },
///        "kind": {
///          "const": "PlayCard"
///        },
///        "position": {
///          "description": "插入站位；缺省追加到最右",
///          "type": "integer",
///          "maximum": 6.0,
///          "minimum": 0.0
///        },
///        "target": {
///          "description": "效果牌或战吼需要目标时填",
///          "$ref": "#/$defs/TargetRef"
///        }
///      },
///      "additionalProperties": false
///    },
///    {
///      "type": "object",
///      "required": [
///        "fromIndex",
///        "kind",
///        "toIndex"
///      ],
///      "properties": {
///        "fromIndex": {
///          "type": "integer",
///          "maximum": 6.0,
///          "minimum": 0.0
///        },
///        "kind": {
///          "const": "RearrangeBoard"
///        },
///        "toIndex": {
///          "type": "integer",
///          "maximum": 6.0,
///          "minimum": 0.0
///        }
///      },
///      "additionalProperties": false
///    },
///    {
///      "type": "object",
///      "required": [
///        "kind"
///      ],
///      "properties": {
///        "kind": {
///          "const": "FreezeShop"
///        },
///        "slot": {
///          "description": "null 表示全部槽位",
///          "type": [
///            "integer",
///            "null"
///          ],
///          "maximum": 5.0,
///          "minimum": 0.0
///        }
///      },
///      "additionalProperties": false
///    },
///    {
///      "type": "object",
///      "required": [
///        "kind"
///      ],
///      "properties": {
///        "kind": {
///          "const": "UnfreezeShop"
///        },
///        "slot": {
///          "description": "null 表示全部槽位",
///          "type": [
///            "integer",
///            "null"
///          ],
///          "maximum": 5.0,
///          "minimum": 0.0
///        }
///      },
///      "additionalProperties": false
///    },
///    {
///      "type": "object",
///      "required": [
///        "kind",
///        "skillId"
///      ],
///      "properties": {
///        "kind": {
///          "const": "UseHeroPower"
///        },
///        "skillId": {
///          "type": "string"
///        },
///        "target": {
///          "description": "需要目标的技能才填",
///          "$ref": "#/$defs/TargetRef"
///        }
///      },
///      "additionalProperties": false
///    },
///    {
///      "type": "object",
///      "required": [
///        "kind"
///      ],
///      "properties": {
///        "kind": {
///          "const": "EndRecruit"
///        }
///      },
///      "additionalProperties": false
///    },
///    {
///      "type": "object",
///      "required": [
///        "choice",
///        "kind"
///      ],
///      "properties": {
///        "choice": {
///          "description": "发现为候选下标，进化为选项下标",
///          "type": "integer",
///          "maximum": 2.0,
///          "minimum": 0.0
///        },
///        "kind": {
///          "const": "ChooseDiscover"
///        }
///      },
///      "additionalProperties": false
///    },
///    {
///      "type": "object",
///      "required": [
///        "kind",
///        "target"
///      ],
///      "properties": {
///        "kind": {
///          "const": "ChooseTarget"
///        },
///        "target": {
///          "description": "目标",
///          "$ref": "#/$defs/TargetRef"
///        }
///      },
///      "additionalProperties": false
///    },
///    {
///      "type": "object",
///      "required": [
///        "kind"
///      ],
///      "properties": {
///        "kind": {
///          "const": "AbandonTarget"
///        }
///      },
///      "additionalProperties": false
///    }
///  ]
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Intent {
    BuyMinion {
        ///商店槽位
        slot: i64,
    },
    SellMinion {
        ///要出售的随从实例 id
        #[serde(rename = "minionId")]
        minion_id: ::std::string::String,
    },
    RerollShop,
    UpgradeEra,
    PlayCard {
        ///手牌位置
        #[serde(rename = "handIndex")]
        hand_index: i64,
        ///插入站位；缺省追加到最右
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        position: ::std::option::Option<i64>,
        ///效果牌或战吼需要目标时填
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        target: ::std::option::Option<TargetRef>,
    },
    RearrangeBoard {
        #[serde(rename = "fromIndex")]
        from_index: i64,
        #[serde(rename = "toIndex")]
        to_index: i64,
    },
    FreezeShop {
        ///null 表示全部槽位
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        slot: ::std::option::Option<i64>,
    },
    UnfreezeShop {
        ///null 表示全部槽位
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        slot: ::std::option::Option<i64>,
    },
    UseHeroPower {
        #[serde(rename = "skillId")]
        skill_id: ::std::string::String,
        ///需要目标的技能才填
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        target: ::std::option::Option<TargetRef>,
    },
    EndRecruit,
    ChooseDiscover {
        ///发现为候选下标，进化为选项下标
        choice: i64,
    },
    ChooseTarget {
        ///目标
        target: TargetRef,
    },
    AbandonTarget,
}
impl ::std::convert::From<&Self> for Intent {
    fn from(value: &Intent) -> Self {
        value.clone()
    }
}
///`IntentMessage`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "clientIntentId",
///    "intent"
///  ],
///  "properties": {
///    "clientIntentId": {
///      "description": "客户端生成，用于幂等",
///      "type": "string",
///      "minLength": 1
///    },
///    "intent": {
///      "$ref": "#/$defs/Intent"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct IntentMessage {
    ///客户端生成，用于幂等
    #[serde(rename = "clientIntentId")]
    pub client_intent_id: IntentMessageClientIntentId,
    pub intent: Intent,
}
impl ::std::convert::From<&IntentMessage> for IntentMessage {
    fn from(value: &IntentMessage) -> Self {
        value.clone()
    }
}
///客户端生成，用于幂等
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "客户端生成，用于幂等",
///  "type": "string",
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct IntentMessageClientIntentId(::std::string::String);
impl ::std::ops::Deref for IntentMessageClientIntentId {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<IntentMessageClientIntentId> for ::std::string::String {
    fn from(value: IntentMessageClientIntentId) -> Self {
        value.0
    }
}
impl ::std::convert::From<&IntentMessageClientIntentId> for IntentMessageClientIntentId {
    fn from(value: &IntentMessageClientIntentId) -> Self {
        value.clone()
    }
}
impl ::std::str::FromStr for IntentMessageClientIntentId {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for IntentMessageClientIntentId {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for IntentMessageClientIntentId {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for IntentMessageClientIntentId {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for IntentMessageClientIntentId {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`InviteAccepted`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "inviteId",
///    "roomCode"
///  ],
///  "properties": {
///    "inviteId": {
///      "type": "string"
///    },
///    "roomCode": {
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct InviteAccepted {
    #[serde(rename = "inviteId")]
    pub invite_id: ::std::string::String,
    #[serde(rename = "roomCode")]
    pub room_code: ::std::string::String,
}
impl ::std::convert::From<&InviteAccepted> for InviteAccepted {
    fn from(value: &InviteAccepted) -> Self {
        value.clone()
    }
}
///`InviteToRoom`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "friendId"
///  ],
///  "properties": {
///    "friendId": {
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct InviteToRoom {
    #[serde(rename = "friendId")]
    pub friend_id: ::std::string::String,
}
impl ::std::convert::From<&InviteToRoom> for InviteToRoom {
    fn from(value: &InviteToRoom) -> Self {
        value.clone()
    }
}
///`JoinQueue`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct JoinQueue {}
impl ::std::convert::From<&JoinQueue> for JoinQueue {
    fn from(value: &JoinQueue) -> Self {
        value.clone()
    }
}
impl ::std::default::Default for JoinQueue {
    fn default() -> Self {
        Self {}
    }
}
///`JoinRoom`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "code"
///  ],
///  "properties": {
///    "code": {
///      "description": "6 位房间码",
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct JoinRoom {
    ///6 位房间码
    pub code: ::std::string::String,
}
impl ::std::convert::From<&JoinRoom> for JoinRoom {
    fn from(value: &JoinRoom) -> Self {
        value.clone()
    }
}
///`KickMember`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "targetPlayerId"
///  ],
///  "properties": {
///    "targetPlayerId": {
///      "description": "要被移出房间的玩家 playerId",
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct KickMember {
    ///要被移出房间的玩家 playerId
    #[serde(rename = "targetPlayerId")]
    pub target_player_id: ::std::string::String,
}
impl ::std::convert::From<&KickMember> for KickMember {
    fn from(value: &KickMember) -> Self {
        value.clone()
    }
}
///`Leaderboard`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "entries"
///  ],
///  "properties": {
///    "entries": {
///      "type": "array",
///      "items": {
///        "type": "object",
///        "required": [
///          "displayName",
///          "playerId",
///          "rank",
///          "rating",
///          "tier"
///        ],
///        "properties": {
///          "displayName": {
///            "type": "string"
///          },
///          "playerId": {
///            "description": "内部不透明标识，用于跳转个人主页",
///            "type": "string"
///          },
///          "rank": {
///            "type": "integer",
///            "maximum": 100.0,
///            "minimum": 1.0
///          },
///          "rating": {
///            "type": "integer",
///            "minimum": 0.0
///          },
///          "tier": {
///            "description": "段位 id",
///            "type": "string"
///          }
///        },
///        "additionalProperties": false
///      }
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Leaderboard {
    pub entries: ::std::vec::Vec<LeaderboardEntriesItem>,
}
impl ::std::convert::From<&Leaderboard> for Leaderboard {
    fn from(value: &Leaderboard) -> Self {
        value.clone()
    }
}
///`LeaderboardEntriesItem`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "displayName",
///    "playerId",
///    "rank",
///    "rating",
///    "tier"
///  ],
///  "properties": {
///    "displayName": {
///      "type": "string"
///    },
///    "playerId": {
///      "description": "内部不透明标识，用于跳转个人主页",
///      "type": "string"
///    },
///    "rank": {
///      "type": "integer",
///      "maximum": 100.0,
///      "minimum": 1.0
///    },
///    "rating": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "tier": {
///      "description": "段位 id",
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct LeaderboardEntriesItem {
    #[serde(rename = "displayName")]
    pub display_name: ::std::string::String,
    ///内部不透明标识，用于跳转个人主页
    #[serde(rename = "playerId")]
    pub player_id: ::std::string::String,
    pub rank: ::std::num::NonZeroU64,
    pub rating: u64,
    ///段位 id
    pub tier: ::std::string::String,
}
impl ::std::convert::From<&LeaderboardEntriesItem> for LeaderboardEntriesItem {
    fn from(value: &LeaderboardEntriesItem) -> Self {
        value.clone()
    }
}
///`LeaveQueue`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct LeaveQueue {}
impl ::std::convert::From<&LeaveQueue> for LeaveQueue {
    fn from(value: &LeaveQueue) -> Self {
        value.clone()
    }
}
impl ::std::default::Default for LeaveQueue {
    fn default() -> Self {
        Self {}
    }
}
///`LeaveRoom`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct LeaveRoom {}
impl ::std::convert::From<&LeaveRoom> for LeaveRoom {
    fn from(value: &LeaveRoom) -> Self {
        value.clone()
    }
}
impl ::std::default::Default for LeaveRoom {
    fn default() -> Self {
        Self {}
    }
}
///`MatchFound`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "roomId",
///    "seat"
///  ],
///  "properties": {
///    "roomId": {
///      "type": "string"
///    },
///    "seat": {
///      "description": "服务端分配的座位号（0 起）",
///      "type": "integer",
///      "maximum": 7.0,
///      "minimum": 0.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MatchFound {
    #[serde(rename = "roomId")]
    pub room_id: ::std::string::String,
    ///服务端分配的座位号（0 起）
    pub seat: i64,
}
impl ::std::convert::From<&MatchFound> for MatchFound {
    fn from(value: &MatchFound) -> Self {
        value.clone()
    }
}
///单项属性的分层构成；在可达的下发状态下 initial + permanent + turn + aura - damage = current
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "单项属性的分层构成；在可达的下发状态下 initial + permanent + turn + aura - damage = current",
///  "type": "object",
///  "required": [
///    "aura",
///    "current",
///    "damage",
///    "initial",
///    "permanent",
///    "turn"
///  ],
///  "properties": {
///    "aura": {
///      "description": "光环派生增量，可为负",
///      "type": "integer"
///    },
///    "current": {
///      "description": "当前实际值",
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "damage": {
///      "description": "层和与当前值之差",
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "initial": {
///      "description": "基础值",
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "permanent": {
///      "description": "成长与永久增量之和",
///      "type": "integer"
///    },
///    "turn": {
///      "description": "本回合临时增量，可为负",
///      "type": "integer"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MinionStatBreakdown {
    ///光环派生增量，可为负
    pub aura: i64,
    ///当前实际值
    pub current: u64,
    ///层和与当前值之差
    pub damage: u64,
    ///基础值
    pub initial: u64,
    ///成长与永久增量之和
    pub permanent: i64,
    ///本回合临时增量，可为负
    pub turn: i64,
}
impl ::std::convert::From<&MinionStatBreakdown> for MinionStatBreakdown {
    fn from(value: &MinionStatBreakdown) -> Self {
        value.clone()
    }
}
///随从某项属性的一条来源；数值为合并后的增量，可为负
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "随从某项属性的一条来源；数值为合并后的增量，可为负",
///  "type": "object",
///  "required": [
///    "attack",
///    "cardId",
///    "health",
///    "kind",
///    "layer",
///    "skillId"
///  ],
///  "properties": {
///    "attack": {
///      "description": "攻击力增量，可为负",
///      "type": "integer"
///    },
///    "cardId": {
///      "description": "来源卡 id；系统来源或卡表缺失时为 null",
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "health": {
///      "description": "生命值增量，可为负",
///      "type": "integer"
///    },
///    "kind": {
///      "description": "来源类型；卡牌效果为 card，其余为规则引擎的系统写入",
///      "type": "string",
///      "enum": [
///        "card",
///        "evolve",
///        "breed",
///        "contagion",
///        "revive",
///        "recall",
///        "combat_growth",
///        "aura_floor",
///        "unknown"
///      ]
///    },
///    "layer": {
///      "description": "来源所属的聚合层；成长已并入 permanent",
///      "type": "string",
///      "enum": [
///        "aura",
///        "permanent",
///        "turn"
///      ]
///    },
///    "skillId": {
///      "description": "来源技能 id；无技能或卡表缺失时为 null",
///      "type": [
///        "string",
///        "null"
///      ]
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MinionStatSource {
    ///攻击力增量，可为负
    pub attack: i64,
    ///来源卡 id；系统来源或卡表缺失时为 null
    #[serde(rename = "cardId")]
    pub card_id: ::std::option::Option<::std::string::String>,
    ///生命值增量，可为负
    pub health: i64,
    ///来源类型；卡牌效果为 card，其余为规则引擎的系统写入
    pub kind: MinionStatSourceKind,
    ///来源所属的聚合层；成长已并入 permanent
    pub layer: MinionStatSourceLayer,
    ///来源技能 id；无技能或卡表缺失时为 null
    #[serde(rename = "skillId")]
    pub skill_id: ::std::option::Option<::std::string::String>,
}
impl ::std::convert::From<&MinionStatSource> for MinionStatSource {
    fn from(value: &MinionStatSource) -> Self {
        value.clone()
    }
}
///来源类型；卡牌效果为 card，其余为规则引擎的系统写入
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "来源类型；卡牌效果为 card，其余为规则引擎的系统写入",
///  "type": "string",
///  "enum": [
///    "card",
///    "evolve",
///    "breed",
///    "contagion",
///    "revive",
///    "recall",
///    "combat_growth",
///    "aura_floor",
///    "unknown"
///  ]
///}
/// ```
/// </details>
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd
)]
pub enum MinionStatSourceKind {
    #[serde(rename = "card")]
    Card,
    #[serde(rename = "evolve")]
    Evolve,
    #[serde(rename = "breed")]
    Breed,
    #[serde(rename = "contagion")]
    Contagion,
    #[serde(rename = "revive")]
    Revive,
    #[serde(rename = "recall")]
    Recall,
    #[serde(rename = "combat_growth")]
    CombatGrowth,
    #[serde(rename = "aura_floor")]
    AuraFloor,
    #[serde(rename = "unknown")]
    Unknown,
}
impl ::std::convert::From<&Self> for MinionStatSourceKind {
    fn from(value: &MinionStatSourceKind) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for MinionStatSourceKind {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Card => f.write_str("card"),
            Self::Evolve => f.write_str("evolve"),
            Self::Breed => f.write_str("breed"),
            Self::Contagion => f.write_str("contagion"),
            Self::Revive => f.write_str("revive"),
            Self::Recall => f.write_str("recall"),
            Self::CombatGrowth => f.write_str("combat_growth"),
            Self::AuraFloor => f.write_str("aura_floor"),
            Self::Unknown => f.write_str("unknown"),
        }
    }
}
impl ::std::str::FromStr for MinionStatSourceKind {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "card" => Ok(Self::Card),
            "evolve" => Ok(Self::Evolve),
            "breed" => Ok(Self::Breed),
            "contagion" => Ok(Self::Contagion),
            "revive" => Ok(Self::Revive),
            "recall" => Ok(Self::Recall),
            "combat_growth" => Ok(Self::CombatGrowth),
            "aura_floor" => Ok(Self::AuraFloor),
            "unknown" => Ok(Self::Unknown),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for MinionStatSourceKind {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for MinionStatSourceKind {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for MinionStatSourceKind {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///来源所属的聚合层；成长已并入 permanent
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "来源所属的聚合层；成长已并入 permanent",
///  "type": "string",
///  "enum": [
///    "aura",
///    "permanent",
///    "turn"
///  ]
///}
/// ```
/// </details>
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd
)]
pub enum MinionStatSourceLayer {
    #[serde(rename = "aura")]
    Aura,
    #[serde(rename = "permanent")]
    Permanent,
    #[serde(rename = "turn")]
    Turn,
}
impl ::std::convert::From<&Self> for MinionStatSourceLayer {
    fn from(value: &MinionStatSourceLayer) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for MinionStatSourceLayer {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Aura => f.write_str("aura"),
            Self::Permanent => f.write_str("permanent"),
            Self::Turn => f.write_str("turn"),
        }
    }
}
impl ::std::str::FromStr for MinionStatSourceLayer {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "aura" => Ok(Self::Aura),
            "permanent" => Ok(Self::Permanent),
            "turn" => Ok(Self::Turn),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for MinionStatSourceLayer {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for MinionStatSourceLayer {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for MinionStatSourceLayer {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///随从攻击与生命的分层构成
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "随从攻击与生命的分层构成",
///  "type": "object",
///  "required": [
///    "attack",
///    "health",
///    "sources"
///  ],
///  "properties": {
///    "attack": {
///      "$ref": "#/$defs/MinionStatBreakdown"
///    },
///    "health": {
///      "$ref": "#/$defs/MinionStatBreakdown"
///    },
///    "sources": {
///      "description": "逐来源加成；按层排序后下发，各层之和等于对应聚合层",
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/MinionStatSource"
///      }
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MinionStats {
    pub attack: MinionStatBreakdown,
    pub health: MinionStatBreakdown,
    ///逐来源加成；按层排序后下发，各层之和等于对应聚合层
    pub sources: ::std::vec::Vec<MinionStatSource>,
}
impl ::std::convert::From<&MinionStats> for MinionStats {
    fn from(value: &MinionStats) -> Self {
        value.clone()
    }
}
///`PendingChoice`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "oneOf": [
///    {
///      "type": "object",
///      "required": [
///        "candidates",
///        "kind"
///      ],
///      "properties": {
///        "candidates": {
///          "description": "三选一的候选 cardId",
///          "type": "array",
///          "items": {
///            "type": "string"
///          }
///        },
///        "kind": {
///          "description": "固定为 discover",
///          "const": "discover"
///        }
///      },
///      "additionalProperties": false
///    },
///    {
///      "type": "object",
///      "required": [
///        "kind",
///        "options"
///      ],
///      "properties": {
///        "kind": {
///          "description": "固定为 evolve",
///          "const": "evolve"
///        },
///        "options": {
///          "description": "进化选项，取值为英文 id",
///          "type": "array",
///          "items": {
///            "type": "string"
///          }
///        }
///      },
///      "additionalProperties": false
///    },
///    {
///      "type": "object",
///      "required": [
///        "candidates",
///        "kind",
///        "sourceMinionId"
///      ],
///      "properties": {
///        "candidates": {
///          "description": "战吼待选的合法目标",
///          "type": "array",
///          "items": {
///            "$ref": "#/$defs/TargetRef"
///          }
///        },
///        "kind": {
///          "description": "固定为 target",
///          "const": "target"
///        },
///        "sourceMinionId": {
///          "description": "触发该待选的随从",
///          "type": "string"
///        }
///      },
///      "additionalProperties": false
///    }
///  ]
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum PendingChoice {
    #[serde(rename = "discover")]
    Discover {
        ///三选一的候选 cardId
        candidates: ::std::vec::Vec<::std::string::String>,
    },
    #[serde(rename = "evolve")]
    Evolve {
        ///进化选项，取值为英文 id
        options: ::std::vec::Vec<::std::string::String>,
    },
    #[serde(rename = "target")]
    Target {
        ///战吼待选的合法目标
        candidates: ::std::vec::Vec<TargetRef>,
        ///触发该待选的随从
        #[serde(rename = "sourceMinionId")]
        source_minion_id: ::std::string::String,
    },
}
impl ::std::convert::From<&Self> for PendingChoice {
    fn from(value: &PendingChoice) -> Self {
        value.clone()
    }
}
///对局阶段
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "对局阶段",
///  "type": "string",
///  "enum": [
///    "Lobby",
///    "Recruit",
///    "Combat",
///    "Settle",
///    "GameOver"
///  ]
///}
/// ```
/// </details>
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd
)]
pub enum Phase {
    Lobby,
    Recruit,
    Combat,
    Settle,
    GameOver,
}
impl ::std::convert::From<&Self> for Phase {
    fn from(value: &Phase) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for Phase {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Lobby => f.write_str("Lobby"),
            Self::Recruit => f.write_str("Recruit"),
            Self::Combat => f.write_str("Combat"),
            Self::Settle => f.write_str("Settle"),
            Self::GameOver => f.write_str("GameOver"),
        }
    }
}
impl ::std::str::FromStr for Phase {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "Lobby" => Ok(Self::Lobby),
            "Recruit" => Ok(Self::Recruit),
            "Combat" => Ok(Self::Combat),
            "Settle" => Ok(Self::Settle),
            "GameOver" => Ok(Self::GameOver),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for Phase {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for Phase {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for Phase {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///`PhaseChanged`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "deadline",
///    "phase",
///    "turn"
///  ],
///  "properties": {
///    "deadline": {
///      "type": "string",
///      "format": "date-time"
///    },
///    "phase": {
///      "$ref": "#/$defs/Phase"
///    },
///    "turn": {
///      "type": "integer",
///      "maximum": 20.0,
///      "minimum": 1.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PhaseChanged {
    pub deadline: ::chrono::DateTime<::chrono::offset::Utc>,
    pub phase: Phase,
    pub turn: ::std::num::NonZeroU64,
}
impl ::std::convert::From<&PhaseChanged> for PhaseChanged {
    fn from(value: &PhaseChanged) -> Self {
        value.clone()
    }
}
///`PhoneCodeSent`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "expiresAt"
///  ],
///  "properties": {
///    "expiresAt": {
///      "description": "验证码失效时刻",
///      "type": "string",
///      "format": "date-time"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PhoneCodeSent {
    ///验证码失效时刻
    #[serde(rename = "expiresAt")]
    pub expires_at: ::chrono::DateTime<::chrono::offset::Utc>,
}
impl ::std::convert::From<&PhoneCodeSent> for PhoneCodeSent {
    fn from(value: &PhoneCodeSent) -> Self {
        value.clone()
    }
}
///`PlayerProfile`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "displayName",
///    "highestTier",
///    "playerId",
///    "rating",
///    "recent",
///    "recentVisible",
///    "tier",
///    "top4Ratio"
///  ],
///  "properties": {
///    "displayName": {
///      "type": "string"
///    },
///    "highestTier": {
///      "description": "历史最高段位 id",
///      "type": "string"
///    },
///    "playerId": {
///      "description": "内部不透明标识，不向玩家展示",
///      "type": "string"
///    },
///    "rating": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "recent": {
///      "type": "array",
///      "items": {
///        "type": "object",
///        "required": [
///          "finishedAt",
///          "placement",
///          "ratingAfter",
///          "ratingDelta"
///        ],
///        "properties": {
///          "finishedAt": {
///            "type": "string",
///            "format": "date-time"
///          },
///          "placement": {
///            "type": "integer",
///            "maximum": 8.0,
///            "minimum": 1.0
///          },
///          "ratingAfter": {
///            "type": "integer",
///            "minimum": 0.0
///          },
///          "ratingDelta": {
///            "description": "实际入账积分变化",
///            "type": "integer"
///          }
///        },
///        "additionalProperties": false
///      },
///      "maxItems": 10
///    },
///    "recentVisible": {
///      "description": "最近 10 局是否对请求者可见；仅本人与好友为 true",
///      "type": "boolean"
///    },
///    "tier": {
///      "description": "当前段位 id",
///      "type": "string"
///    },
///    "top4Ratio": {
///      "description": "最近 10 局中名次前 4 的比例；不可见或无记录时为 0",
///      "type": "number",
///      "maximum": 1.0,
///      "minimum": 0.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PlayerProfile {
    #[serde(rename = "displayName")]
    pub display_name: ::std::string::String,
    ///历史最高段位 id
    #[serde(rename = "highestTier")]
    pub highest_tier: ::std::string::String,
    ///内部不透明标识，不向玩家展示
    #[serde(rename = "playerId")]
    pub player_id: ::std::string::String,
    pub rating: u64,
    pub recent: ::std::vec::Vec<PlayerProfileRecentItem>,
    ///最近 10 局是否对请求者可见；仅本人与好友为 true
    #[serde(rename = "recentVisible")]
    pub recent_visible: bool,
    ///当前段位 id
    pub tier: ::std::string::String,
    #[serde(rename = "top4Ratio")]
    pub top4_ratio: f64,
}
impl ::std::convert::From<&PlayerProfile> for PlayerProfile {
    fn from(value: &PlayerProfile) -> Self {
        value.clone()
    }
}
///`PlayerProfileRecentItem`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "finishedAt",
///    "placement",
///    "ratingAfter",
///    "ratingDelta"
///  ],
///  "properties": {
///    "finishedAt": {
///      "type": "string",
///      "format": "date-time"
///    },
///    "placement": {
///      "type": "integer",
///      "maximum": 8.0,
///      "minimum": 1.0
///    },
///    "ratingAfter": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "ratingDelta": {
///      "description": "实际入账积分变化",
///      "type": "integer"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PlayerProfileRecentItem {
    #[serde(rename = "finishedAt")]
    pub finished_at: ::chrono::DateTime<::chrono::offset::Utc>,
    pub placement: ::std::num::NonZeroU64,
    #[serde(rename = "ratingAfter")]
    pub rating_after: u64,
    ///实际入账积分变化
    #[serde(rename = "ratingDelta")]
    pub rating_delta: i64,
}
impl ::std::convert::From<&PlayerProfileRecentItem> for PlayerProfileRecentItem {
    fn from(value: &PlayerProfileRecentItem) -> Self {
        value.clone()
    }
}
///`PlayerSummary`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "alive",
///    "boardCount",
///    "displayName",
///    "era",
///    "heroCardId",
///    "hp",
///    "managed",
///    "playerId",
///    "rank",
///    "recruited",
///    "seat"
///  ],
///  "properties": {
///    "alive": {
///      "type": "boolean"
///    },
///    "boardCount": {
///      "type": "integer",
///      "maximum": 7.0,
///      "minimum": 0.0
///    },
///    "displayName": {
///      "type": "string"
///    },
///    "era": {
///      "type": "integer",
///      "maximum": 7.0,
///      "minimum": 1.0
///    },
///    "heroCardId": {
///      "description": "开局选择前为 null",
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "hp": {
///      "type": "integer"
///    },
///    "managed": {
///      "description": "当前是否由 AI 托管",
///      "type": "boolean"
///    },
///    "placement": {
///      "description": "最终名次，对局结束前为 null",
///      "type": [
///        "integer",
///        "null"
///      ]
///    },
///    "playerId": {
///      "type": "string"
///    },
///    "rank": {
///      "description": "当前名次；对局结束前为实时名次，结束后等于 placement",
///      "type": [
///        "integer",
///        "null"
///      ],
///      "maximum": 8.0,
///      "minimum": 1.0
///    },
///    "recruited": {
///      "description": "本回合是否已结束招募",
///      "type": "boolean"
///    },
///    "seat": {
///      "type": "integer",
///      "maximum": 7.0,
///      "minimum": 0.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PlayerSummary {
    pub alive: bool,
    #[serde(rename = "boardCount")]
    pub board_count: i64,
    #[serde(rename = "displayName")]
    pub display_name: ::std::string::String,
    pub era: ::std::num::NonZeroU64,
    ///开局选择前为 null
    #[serde(rename = "heroCardId")]
    pub hero_card_id: ::std::option::Option<::std::string::String>,
    pub hp: i64,
    ///当前是否由 AI 托管
    pub managed: bool,
    ///最终名次，对局结束前为 null
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub placement: ::std::option::Option<i64>,
    #[serde(rename = "playerId")]
    pub player_id: ::std::string::String,
    ///当前名次；对局结束前为实时名次，结束后等于 placement
    pub rank: ::std::option::Option<::std::num::NonZeroU64>,
    ///本回合是否已结束招募
    pub recruited: bool,
    pub seat: i64,
}
impl ::std::convert::From<&PlayerSummary> for PlayerSummary {
    fn from(value: &PlayerSummary) -> Self {
        value.clone()
    }
}
///`PlayerView`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "boardSlots",
///    "combatLog",
///    "enabledFactions",
///    "hpCap",
///    "phase",
///    "phaseDeadline",
///    "players",
///    "roomId",
///    "self",
///    "turn"
///  ],
///  "properties": {
///    "boardSlots": {
///      "description": "本方棋盘随从位上限，v0.1 为 7",
///      "type": "integer",
///      "minimum": 1.0
///    },
///    "combatLog": {
///      "description": "本局最近 5 个回合的全部战斗，按回合升序；全对局公开",
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/CombatLogEntry"
///      }
///    },
///    "enabledFactions": {
///      "description": "本局启用的 5 个玩法",
///      "type": "array",
///      "items": {
///        "type": "string"
///      },
///      "maxItems": 5,
///      "minItems": 5
///    },
///    "hpCap": {
///      "description": "生命上限，v0.1 为 40",
///      "type": "integer",
///      "minimum": 1.0
///    },
///    "opponentPlayerId": {
///      "description": "当前战斗对手；招募阶段或未配对时为 null",
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "phase": {
///      "$ref": "#/$defs/Phase"
///    },
///    "phaseDeadline": {
///      "type": "string",
///      "format": "date-time"
///    },
///    "players": {
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/PlayerSummary"
///      }
///    },
///    "roomId": {
///      "type": "string"
///    },
///    "self": {
///      "type": "object",
///      "required": [
///        "affordances",
///        "board",
///        "era",
///        "gold",
///        "hand",
///        "heroCardId",
///        "heroPowers",
///        "hp",
///        "pendingChoices",
///        "playerId",
///        "recruited",
///        "rerollCost",
///        "seat",
///        "shop",
///        "upgradeCost"
///      ],
///      "properties": {
///        "affordances": {
///          "$ref": "#/$defs/Affordances"
///        },
///        "board": {
///          "type": "array",
///          "items": {
///            "$ref": "#/$defs/BoardMinion"
///          }
///        },
///        "era": {
///          "type": "integer",
///          "maximum": 7.0,
///          "minimum": 1.0
///        },
///        "gold": {
///          "type": "integer",
///          "minimum": 0.0
///        },
///        "hand": {
///          "type": "array",
///          "items": {
///            "$ref": "#/$defs/HandCard"
///          }
///        },
///        "heroCardId": {
///          "type": [
///            "string",
///            "null"
///          ]
///        },
///        "heroPowers": {
///          "description": "英雄卡上的全部主动技能，按英雄卡 skills 顺序排列；没有主动技能时为空",
///          "type": "array",
///          "items": {
///            "$ref": "#/$defs/HeroPower"
///          }
///        },
///        "hp": {
///          "type": "integer"
///        },
///        "pendingChoices": {
///          "description": "本座位待处理的发现或进化选择，按先到先处理排序",
///          "type": "array",
///          "items": {
///            "$ref": "#/$defs/PendingChoice"
///          }
///        },
///        "playerId": {
///          "type": "string"
///        },
///        "recruited": {
///          "type": "boolean"
///        },
///        "rerollCost": {
///          "type": "integer",
///          "minimum": 0.0
///        },
///        "seat": {
///          "type": "integer",
///          "maximum": 7.0,
///          "minimum": 0.0
///        },
///        "shop": {
///          "type": "array",
///          "items": {
///            "$ref": "#/$defs/ShopSlot"
///          }
///        },
///        "upgradeCost": {
///          "description": "满时代为 null",
///          "type": [
///            "integer",
///            "null"
///          ],
///          "minimum": 2.0
///        }
///      },
///      "additionalProperties": false
///    },
///    "turn": {
///      "type": "integer",
///      "maximum": 20.0,
///      "minimum": 1.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PlayerView {
    ///本方棋盘随从位上限，v0.1 为 7
    #[serde(rename = "boardSlots")]
    pub board_slots: ::std::num::NonZeroU64,
    ///本局最近 5 个回合的全部战斗，按回合升序；全对局公开
    #[serde(rename = "combatLog")]
    pub combat_log: ::std::vec::Vec<CombatLogEntry>,
    ///本局启用的 5 个玩法
    #[serde(rename = "enabledFactions")]
    pub enabled_factions: [::std::string::String; 5usize],
    ///生命上限，v0.1 为 40
    #[serde(rename = "hpCap")]
    pub hp_cap: ::std::num::NonZeroU64,
    ///当前战斗对手；招募阶段或未配对时为 null
    #[serde(
        rename = "opponentPlayerId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub opponent_player_id: ::std::option::Option<::std::string::String>,
    pub phase: Phase,
    #[serde(rename = "phaseDeadline")]
    pub phase_deadline: ::chrono::DateTime<::chrono::offset::Utc>,
    pub players: ::std::vec::Vec<PlayerSummary>,
    #[serde(rename = "roomId")]
    pub room_id: ::std::string::String,
    #[serde(rename = "self")]
    pub self_: PlayerViewSelf,
    pub turn: ::std::num::NonZeroU64,
}
impl ::std::convert::From<&PlayerView> for PlayerView {
    fn from(value: &PlayerView) -> Self {
        value.clone()
    }
}
///`PlayerViewSelf`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "affordances",
///    "board",
///    "era",
///    "gold",
///    "hand",
///    "heroCardId",
///    "heroPowers",
///    "hp",
///    "pendingChoices",
///    "playerId",
///    "recruited",
///    "rerollCost",
///    "seat",
///    "shop",
///    "upgradeCost"
///  ],
///  "properties": {
///    "affordances": {
///      "$ref": "#/$defs/Affordances"
///    },
///    "board": {
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/BoardMinion"
///      }
///    },
///    "era": {
///      "type": "integer",
///      "maximum": 7.0,
///      "minimum": 1.0
///    },
///    "gold": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "hand": {
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/HandCard"
///      }
///    },
///    "heroCardId": {
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "heroPowers": {
///      "description": "英雄卡上的全部主动技能，按英雄卡 skills 顺序排列；没有主动技能时为空",
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/HeroPower"
///      }
///    },
///    "hp": {
///      "type": "integer"
///    },
///    "pendingChoices": {
///      "description": "本座位待处理的发现或进化选择，按先到先处理排序",
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/PendingChoice"
///      }
///    },
///    "playerId": {
///      "type": "string"
///    },
///    "recruited": {
///      "type": "boolean"
///    },
///    "rerollCost": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "seat": {
///      "type": "integer",
///      "maximum": 7.0,
///      "minimum": 0.0
///    },
///    "shop": {
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/ShopSlot"
///      }
///    },
///    "upgradeCost": {
///      "description": "满时代为 null",
///      "type": [
///        "integer",
///        "null"
///      ],
///      "minimum": 2.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PlayerViewSelf {
    pub affordances: Affordances,
    pub board: ::std::vec::Vec<BoardMinion>,
    pub era: ::std::num::NonZeroU64,
    pub gold: u64,
    pub hand: ::std::vec::Vec<HandCard>,
    #[serde(rename = "heroCardId")]
    pub hero_card_id: ::std::option::Option<::std::string::String>,
    ///英雄卡上的全部主动技能，按英雄卡 skills 顺序排列；没有主动技能时为空
    #[serde(rename = "heroPowers")]
    pub hero_powers: ::std::vec::Vec<HeroPower>,
    pub hp: i64,
    ///本座位待处理的发现或进化选择，按先到先处理排序
    #[serde(rename = "pendingChoices")]
    pub pending_choices: ::std::vec::Vec<PendingChoice>,
    #[serde(rename = "playerId")]
    pub player_id: ::std::string::String,
    pub recruited: bool,
    #[serde(rename = "rerollCost")]
    pub reroll_cost: u64,
    pub seat: i64,
    pub shop: ::std::vec::Vec<ShopSlot>,
    ///满时代为 null
    #[serde(rename = "upgradeCost")]
    pub upgrade_cost: ::std::option::Option<i64>,
}
impl ::std::convert::From<&PlayerViewSelf> for PlayerViewSelf {
    fn from(value: &PlayerViewSelf) -> Self {
        value.clone()
    }
}
///`QueueState`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "ratingWindow",
///    "waitingMs",
///    "waitingPlayers"
///  ],
///  "properties": {
///    "ratingWindow": {
///      "description": "当前匹配积分半径；null 表示不限积分",
///      "type": [
///        "integer",
///        "null"
///      ],
///      "minimum": 0.0
///    },
///    "waitingMs": {
///      "description": "本人已等待时长（毫秒）",
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "waitingPlayers": {
///      "description": "当前队列里的玩家数，客户端显示为「已凑 X/8」",
///      "type": "integer",
///      "maximum": 8.0,
///      "minimum": 0.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct QueueState {
    ///当前匹配积分半径；null 表示不限积分
    #[serde(rename = "ratingWindow")]
    pub rating_window: ::std::option::Option<u64>,
    ///本人已等待时长（毫秒）
    #[serde(rename = "waitingMs")]
    pub waiting_ms: u64,
    ///当前队列里的玩家数，客户端显示为「已凑 X/8」
    #[serde(rename = "waitingPlayers")]
    pub waiting_players: i64,
}
impl ::std::convert::From<&QueueState> for QueueState {
    fn from(value: &QueueState) -> Self {
        value.clone()
    }
}
///`Ready`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "ready"
///  ],
///  "properties": {
///    "ready": {
///      "type": "boolean"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Ready {
    pub ready: bool,
}
impl ::std::convert::From<&Ready> for Ready {
    fn from(value: &Ready) -> Self {
        value.clone()
    }
}
///`Reconnect`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "lastSeq",
///    "roomId"
///  ],
///  "properties": {
///    "lastSeq": {
///      "description": "客户端最后收到的 seq",
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "roomId": {
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Reconnect {
    ///客户端最后收到的 seq
    #[serde(rename = "lastSeq")]
    pub last_seq: u64,
    #[serde(rename = "roomId")]
    pub room_id: ::std::string::String,
}
impl ::std::convert::From<&Reconnect> for Reconnect {
    fn from(value: &Reconnect) -> Self {
        value.clone()
    }
}
///`RedeemRoomInvite`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "token"
///  ],
///  "properties": {
///    "token": {
///      "description": "一次性邀请 token",
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RedeemRoomInvite {
    ///一次性邀请 token
    pub token: ::std::string::String,
}
impl ::std::convert::From<&RedeemRoomInvite> for RedeemRoomInvite {
    fn from(value: &RedeemRoomInvite) -> Self {
        value.clone()
    }
}
///`RequestAccount`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RequestAccount {}
impl ::std::convert::From<&RequestAccount> for RequestAccount {
    fn from(value: &RequestAccount) -> Self {
        value.clone()
    }
}
impl ::std::default::Default for RequestAccount {
    fn default() -> Self {
        Self {}
    }
}
///`RequestLeaderboard`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RequestLeaderboard {}
impl ::std::convert::From<&RequestLeaderboard> for RequestLeaderboard {
    fn from(value: &RequestLeaderboard) -> Self {
        value.clone()
    }
}
impl ::std::default::Default for RequestLeaderboard {
    fn default() -> Self {
        Self {}
    }
}
///`RequestPhoneCode`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "phone"
///  ],
///  "properties": {
///    "phone": {
///      "description": "待绑定的手机号",
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RequestPhoneCode {
    ///待绑定的手机号
    pub phone: ::std::string::String,
}
impl ::std::convert::From<&RequestPhoneCode> for RequestPhoneCode {
    fn from(value: &RequestPhoneCode) -> Self {
        value.clone()
    }
}
///`RequestProfile`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "properties": {
///    "playerId": {
///      "description": "要查看的玩家；缺省为本人",
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RequestProfile {
    ///要查看的玩家；缺省为本人
    #[serde(
        rename = "playerId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub player_id: ::std::option::Option<::std::string::String>,
}
impl ::std::convert::From<&RequestProfile> for RequestProfile {
    fn from(value: &RequestProfile) -> Self {
        value.clone()
    }
}
impl ::std::default::Default for RequestProfile {
    fn default() -> Self {
        Self {
            player_id: Default::default(),
        }
    }
}
///`RequestSnapshot`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RequestSnapshot {}
impl ::std::convert::From<&RequestSnapshot> for RequestSnapshot {
    fn from(value: &RequestSnapshot) -> Self {
        value.clone()
    }
}
impl ::std::default::Default for RequestSnapshot {
    fn default() -> Self {
        Self {}
    }
}
///`RespondInvite`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "accept",
///    "inviteId"
///  ],
///  "properties": {
///    "accept": {
///      "type": "boolean"
///    },
///    "inviteId": {
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RespondInvite {
    pub accept: bool,
    #[serde(rename = "inviteId")]
    pub invite_id: ::std::string::String,
}
impl ::std::convert::From<&RespondInvite> for RespondInvite {
    fn from(value: &RespondInvite) -> Self {
        value.clone()
    }
}
///`RoomClosed`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "reason"
///  ],
///  "properties": {
///    "reason": {
///      "description": "kicked：被房主移出；dissolved：房间已解散",
///      "type": "string",
///      "enum": [
///        "kicked",
///        "dissolved"
///      ]
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RoomClosed {
    ///kicked：被房主移出；dissolved：房间已解散
    pub reason: RoomClosedReason,
}
impl ::std::convert::From<&RoomClosed> for RoomClosed {
    fn from(value: &RoomClosed) -> Self {
        value.clone()
    }
}
///kicked：被房主移出；dissolved：房间已解散
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "kicked：被房主移出；dissolved：房间已解散",
///  "type": "string",
///  "enum": [
///    "kicked",
///    "dissolved"
///  ]
///}
/// ```
/// </details>
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd
)]
pub enum RoomClosedReason {
    #[serde(rename = "kicked")]
    Kicked,
    #[serde(rename = "dissolved")]
    Dissolved,
}
impl ::std::convert::From<&Self> for RoomClosedReason {
    fn from(value: &RoomClosedReason) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for RoomClosedReason {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Kicked => f.write_str("kicked"),
            Self::Dissolved => f.write_str("dissolved"),
        }
    }
}
impl ::std::str::FromStr for RoomClosedReason {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "kicked" => Ok(Self::Kicked),
            "dissolved" => Ok(Self::Dissolved),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for RoomClosedReason {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for RoomClosedReason {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for RoomClosedReason {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///`RoomInviteCode`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "expiresAt",
///    "roomCode",
///    "token"
///  ],
///  "properties": {
///    "expiresAt": {
///      "description": "token 失效时刻，创建后 60 秒",
///      "type": "string",
///      "format": "date-time"
///    },
///    "roomCode": {
///      "description": "6 位房间码",
///      "type": "string"
///    },
///    "token": {
///      "description": "一次性邀请 token",
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RoomInviteCode {
    ///token 失效时刻，创建后 60 秒
    #[serde(rename = "expiresAt")]
    pub expires_at: ::chrono::DateTime<::chrono::offset::Utc>,
    ///6 位房间码
    #[serde(rename = "roomCode")]
    pub room_code: ::std::string::String,
    ///一次性邀请 token
    pub token: ::std::string::String,
}
impl ::std::convert::From<&RoomInviteCode> for RoomInviteCode {
    fn from(value: &RoomInviteCode) -> Self {
        value.clone()
    }
}
///`RoomUpdate`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "botFill",
///    "code",
///    "hostId",
///    "phase",
///    "players",
///    "roomId"
///  ],
///  "properties": {
///    "botFill": {
///      "description": "开始时空位是否用机器人补满",
///      "type": "boolean"
///    },
///    "code": {
///      "description": "6 位房间码",
///      "type": "string"
///    },
///    "hostId": {
///      "type": "string"
///    },
///    "phase": {
///      "$ref": "#/$defs/Phase"
///    },
///    "players": {
///      "type": "array",
///      "items": {
///        "type": "object",
///        "required": [
///          "displayName",
///          "isHost",
///          "playerId",
///          "ready",
///          "seat"
///        ],
///        "properties": {
///          "displayName": {
///            "type": "string"
///          },
///          "isHost": {
///            "type": "boolean"
///          },
///          "playerId": {
///            "type": "string"
///          },
///          "ready": {
///            "type": "boolean"
///          },
///          "seat": {
///            "type": "integer",
///            "maximum": 7.0,
///            "minimum": 0.0
///          }
///        },
///        "additionalProperties": false
///      }
///    },
///    "roomId": {
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RoomUpdate {
    ///开始时空位是否用机器人补满
    #[serde(rename = "botFill")]
    pub bot_fill: bool,
    ///6 位房间码
    pub code: ::std::string::String,
    #[serde(rename = "hostId")]
    pub host_id: ::std::string::String,
    pub phase: Phase,
    pub players: ::std::vec::Vec<RoomUpdatePlayersItem>,
    #[serde(rename = "roomId")]
    pub room_id: ::std::string::String,
}
impl ::std::convert::From<&RoomUpdate> for RoomUpdate {
    fn from(value: &RoomUpdate) -> Self {
        value.clone()
    }
}
///`RoomUpdatePlayersItem`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "displayName",
///    "isHost",
///    "playerId",
///    "ready",
///    "seat"
///  ],
///  "properties": {
///    "displayName": {
///      "type": "string"
///    },
///    "isHost": {
///      "type": "boolean"
///    },
///    "playerId": {
///      "type": "string"
///    },
///    "ready": {
///      "type": "boolean"
///    },
///    "seat": {
///      "type": "integer",
///      "maximum": 7.0,
///      "minimum": 0.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RoomUpdatePlayersItem {
    #[serde(rename = "displayName")]
    pub display_name: ::std::string::String,
    #[serde(rename = "isHost")]
    pub is_host: bool,
    #[serde(rename = "playerId")]
    pub player_id: ::std::string::String,
    pub ready: bool,
    pub seat: i64,
}
impl ::std::convert::From<&RoomUpdatePlayersItem> for RoomUpdatePlayersItem {
    fn from(value: &RoomUpdatePlayersItem) -> Self {
        value.clone()
    }
}
///`SelectHeroCard`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "cardId"
///  ],
///  "properties": {
///    "cardId": {
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct SelectHeroCard {
    #[serde(rename = "cardId")]
    pub card_id: ::std::string::String,
}
impl ::std::convert::From<&SelectHeroCard> for SelectHeroCard {
    fn from(value: &SelectHeroCard) -> Self {
        value.clone()
    }
}
///`SelfStats`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "board",
///    "buys",
///    "combatDraws",
///    "combatLosses",
///    "combatWins",
///    "damageDealt",
///    "damageTaken",
///    "finalEra",
///    "finalHp",
///    "heroPowerUses",
///    "place",
///    "remainingGold",
///    "rerolls",
///    "sells",
///    "survivedTurns",
///    "triples"
///  ],
///  "properties": {
///    "board": {
///      "description": "终局棋盘，按站位顺序；最多 7 项",
///      "type": "array",
///      "items": {
///        "type": "object",
///        "required": [
///          "attack",
///          "cardId",
///          "golden",
///          "health"
///        ],
///        "properties": {
///          "attack": {
///            "type": "integer",
///            "minimum": 0.0
///          },
///          "cardId": {
///            "type": "string"
///          },
///          "golden": {
///            "type": "boolean"
///          },
///          "health": {
///            "type": "integer",
///            "minimum": 0.0
///          }
///        },
///        "additionalProperties": false
///      },
///      "maxItems": 7
///    },
///    "buys": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "combatDraws": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "combatLosses": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "combatWins": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "damageDealt": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "damageTaken": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "finalEra": {
///      "type": "integer",
///      "maximum": 7.0,
///      "minimum": 1.0
///    },
///    "finalHp": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "heroPowerUses": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "place": {
///      "type": "integer",
///      "maximum": 8.0,
///      "minimum": 1.0
///    },
///    "remainingGold": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "rerolls": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "sells": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "survivedTurns": {
///      "type": "integer",
///      "maximum": 20.0,
///      "minimum": 0.0
///    },
///    "triples": {
///      "type": "integer",
///      "minimum": 0.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct SelfStats {
    ///终局棋盘，按站位顺序；最多 7 项
    pub board: ::std::vec::Vec<SelfStatsBoardItem>,
    pub buys: u64,
    #[serde(rename = "combatDraws")]
    pub combat_draws: u64,
    #[serde(rename = "combatLosses")]
    pub combat_losses: u64,
    #[serde(rename = "combatWins")]
    pub combat_wins: u64,
    #[serde(rename = "damageDealt")]
    pub damage_dealt: u64,
    #[serde(rename = "damageTaken")]
    pub damage_taken: u64,
    #[serde(rename = "finalEra")]
    pub final_era: ::std::num::NonZeroU64,
    #[serde(rename = "finalHp")]
    pub final_hp: u64,
    #[serde(rename = "heroPowerUses")]
    pub hero_power_uses: u64,
    pub place: ::std::num::NonZeroU64,
    #[serde(rename = "remainingGold")]
    pub remaining_gold: u64,
    pub rerolls: u64,
    pub sells: u64,
    #[serde(rename = "survivedTurns")]
    pub survived_turns: i64,
    pub triples: u64,
}
impl ::std::convert::From<&SelfStats> for SelfStats {
    fn from(value: &SelfStats) -> Self {
        value.clone()
    }
}
///`SelfStatsBoardItem`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "attack",
///    "cardId",
///    "golden",
///    "health"
///  ],
///  "properties": {
///    "attack": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "cardId": {
///      "type": "string"
///    },
///    "golden": {
///      "type": "boolean"
///    },
///    "health": {
///      "type": "integer",
///      "minimum": 0.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct SelfStatsBoardItem {
    pub attack: u64,
    #[serde(rename = "cardId")]
    pub card_id: ::std::string::String,
    pub golden: bool,
    pub health: u64,
}
impl ::std::convert::From<&SelfStatsBoardItem> for SelfStatsBoardItem {
    fn from(value: &SelfStatsBoardItem) -> Self {
        value.clone()
    }
}
///`ServerEnvelope`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "id",
///    "payload",
///    "seq",
///    "ts",
///    "type",
///    "v"
///  ],
///  "properties": {
///    "id": {
///      "description": "服务端生成的唯一消息 id",
///      "type": "string",
///      "minLength": 1
///    },
///    "payload": {
///      "description": "由 type 决定的结构",
///      "type": "object"
///    },
///    "playerId": {
///      "description": "接收者 playerId",
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "replyTo": {
///      "description": "回复的上行消息 id",
///      "type": "string"
///    },
///    "roomId": {
///      "description": "未进房间时为 null",
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "seq": {
///      "description": "发给该玩家的消息序号，单调递增",
///      "type": "integer",
///      "minimum": 1.0
///    },
///    "ts": {
///      "description": "RFC 3339 UTC 时间",
///      "type": "string",
///      "format": "date-time"
///    },
///    "type": {
///      "description": "消息类型，唯一路由键",
///      "type": "string",
///      "minLength": 1
///    },
///    "v": {
///      "description": "协议主版本",
///      "const": 1
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ServerEnvelope {
    ///服务端生成的唯一消息 id
    pub id: ServerEnvelopeId,
    ///由 type 决定的结构
    pub payload: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
    ///接收者 playerId
    #[serde(
        rename = "playerId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub player_id: ::std::option::Option<::std::string::String>,
    ///回复的上行消息 id
    #[serde(
        rename = "replyTo",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub reply_to: ::std::option::Option<::std::string::String>,
    ///未进房间时为 null
    #[serde(
        rename = "roomId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub room_id: ::std::option::Option<::std::string::String>,
    ///发给该玩家的消息序号，单调递增
    pub seq: ::std::num::NonZeroU64,
    ///RFC 3339 UTC 时间
    pub ts: ::chrono::DateTime<::chrono::offset::Utc>,
    ///消息类型，唯一路由键
    #[serde(rename = "type")]
    pub type_: ServerEnvelopeType,
    ///协议主版本
    pub v: ::serde_json::Value,
}
impl ::std::convert::From<&ServerEnvelope> for ServerEnvelope {
    fn from(value: &ServerEnvelope) -> Self {
        value.clone()
    }
}
///服务端生成的唯一消息 id
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "服务端生成的唯一消息 id",
///  "type": "string",
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ServerEnvelopeId(::std::string::String);
impl ::std::ops::Deref for ServerEnvelopeId {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ServerEnvelopeId> for ::std::string::String {
    fn from(value: ServerEnvelopeId) -> Self {
        value.0
    }
}
impl ::std::convert::From<&ServerEnvelopeId> for ServerEnvelopeId {
    fn from(value: &ServerEnvelopeId) -> Self {
        value.clone()
    }
}
impl ::std::str::FromStr for ServerEnvelopeId {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ServerEnvelopeId {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ServerEnvelopeId {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ServerEnvelopeId {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ServerEnvelopeId {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///消息类型，唯一路由键
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "消息类型，唯一路由键",
///  "type": "string",
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ServerEnvelopeType(::std::string::String);
impl ::std::ops::Deref for ServerEnvelopeType {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ServerEnvelopeType> for ::std::string::String {
    fn from(value: ServerEnvelopeType) -> Self {
        value.0
    }
}
impl ::std::convert::From<&ServerEnvelopeType> for ServerEnvelopeType {
    fn from(value: &ServerEnvelopeType) -> Self {
        value.clone()
    }
}
impl ::std::str::FromStr for ServerEnvelopeType {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ServerEnvelopeType {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ServerEnvelopeType {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ServerEnvelopeType {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ServerEnvelopeType {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`SetBotFill`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "enabled"
///  ],
///  "properties": {
///    "enabled": {
///      "description": "是否在开始时用机器人补满空位",
///      "type": "boolean"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct SetBotFill {
    ///是否在开始时用机器人补满空位
    pub enabled: bool,
}
impl ::std::convert::From<&SetBotFill> for SetBotFill {
    fn from(value: &SetBotFill) -> Self {
        value.clone()
    }
}
///`ShopAffordance`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "buyable",
///    "disabledReason",
///    "index"
///  ],
///  "properties": {
///    "buyable": {
///      "type": "boolean"
///    },
///    "disabledReason": {
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "index": {
///      "type": "integer",
///      "maximum": 5.0,
///      "minimum": 0.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ShopAffordance {
    pub buyable: bool,
    #[serde(rename = "disabledReason")]
    pub disabled_reason: ::std::option::Option<::std::string::String>,
    pub index: i64,
}
impl ::std::convert::From<&ShopAffordance> for ShopAffordance {
    fn from(value: &ShopAffordance) -> Self {
        value.clone()
    }
}
///`ShopSlot`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "cardId",
///    "frozen",
///    "index",
///    "instanceId"
///  ],
///  "properties": {
///    "cardId": {
///      "description": "空槽位为 null",
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "frozen": {
///      "type": "boolean"
///    },
///    "index": {
///      "type": "integer",
///      "maximum": 5.0,
///      "minimum": 0.0
///    },
///    "instanceId": {
///      "description": "卡牌实例编号；空槽位为 null",
///      "type": [
///        "string",
///        "null"
///      ]
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ShopSlot {
    ///空槽位为 null
    #[serde(rename = "cardId")]
    pub card_id: ::std::option::Option<::std::string::String>,
    pub frozen: bool,
    pub index: i64,
    ///卡牌实例编号；空槽位为 null
    #[serde(rename = "instanceId")]
    pub instance_id: ::std::option::Option<::std::string::String>,
}
impl ::std::convert::From<&ShopSlot> for ShopSlot {
    fn from(value: &ShopSlot) -> Self {
        value.clone()
    }
}
///`Snapshot`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "view"
///  ],
///  "properties": {
///    "seq": {
///      "description": "快照对应的座位消息序号；与信封 seq 一致，M1 新增可选字段",
///      "type": "integer",
///      "minimum": 1.0
///    },
///    "view": {
///      "$ref": "#/$defs/PlayerView"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    ///快照对应的座位消息序号；与信封 seq 一致，M1 新增可选字段
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub seq: ::std::option::Option<::std::num::NonZeroU64>,
    pub view: PlayerView,
}
impl ::std::convert::From<&Snapshot> for Snapshot {
    fn from(value: &Snapshot) -> Self {
        value.clone()
    }
}
///`StartGame`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct StartGame {}
impl ::std::convert::From<&StartGame> for StartGame {
    fn from(value: &StartGame) -> Self {
        value.clone()
    }
}
impl ::std::default::Default for StartGame {
    fn default() -> Self {
        Self {}
    }
}
///`StateUpdate`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "events",
///    "view"
///  ],
///  "properties": {
///    "events": {
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/GameEvent"
///      }
///    },
///    "view": {
///      "$ref": "#/$defs/PlayerView"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct StateUpdate {
    pub events: ::std::vec::Vec<GameEvent>,
    pub view: PlayerView,
}
impl ::std::convert::From<&StateUpdate> for StateUpdate {
    fn from(value: &StateUpdate) -> Self {
        value.clone()
    }
}
///`TargetRef`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "id",
///    "zone"
///  ],
///  "properties": {
///    "id": {
///      "description": "目标实例编号；三个区域共用同一编号空间",
///      "type": "string"
///    },
///    "zone": {
///      "$ref": "#/$defs/TargetZone"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct TargetRef {
    ///目标实例编号；三个区域共用同一编号空间
    pub id: ::std::string::String,
    pub zone: TargetZone,
}
impl ::std::convert::From<&TargetRef> for TargetRef {
    fn from(value: &TargetRef) -> Self {
        value.clone()
    }
}
///目标所在区域：本人手牌、本人商店、我方棋盘
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "目标所在区域：本人手牌、本人商店、我方棋盘",
///  "type": "string",
///  "enum": [
///    "hand",
///    "shop",
///    "board"
///  ]
///}
/// ```
/// </details>
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd
)]
pub enum TargetZone {
    #[serde(rename = "hand")]
    Hand,
    #[serde(rename = "shop")]
    Shop,
    #[serde(rename = "board")]
    Board,
}
impl ::std::convert::From<&Self> for TargetZone {
    fn from(value: &TargetZone) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for TargetZone {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Hand => f.write_str("hand"),
            Self::Shop => f.write_str("shop"),
            Self::Board => f.write_str("board"),
        }
    }
}
impl ::std::str::FromStr for TargetZone {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "hand" => Ok(Self::Hand),
            "shop" => Ok(Self::Shop),
            "board" => Ok(Self::Board),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for TargetZone {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for TargetZone {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for TargetZone {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///`TimerWarning`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "secondsLeft"
///  ],
///  "properties": {
///    "secondsLeft": {
///      "type": "integer",
///      "minimum": 0.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct TimerWarning {
    #[serde(rename = "secondsLeft")]
    pub seconds_left: u64,
}
impl ::std::convert::From<&TimerWarning> for TimerWarning {
    fn from(value: &TimerWarning) -> Self {
        value.clone()
    }
}
