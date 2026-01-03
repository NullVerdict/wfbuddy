#[derive(serde::Deserialize)]
pub struct Sentinels {
	#[serde(rename = "ExportSentinels")]
	pub sentinels: Vec<Sentinel>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sentinel {
	pub unique_name: String,
	pub name: String,
}

// {
// 	"uniqueName": "/Lotus/Powersuits/Khora/Kavat/KhoraPrimeKavatPowerSuit",
// 	"name": "Venari Prime",
// 	"health": 1050,
// 	"shield": 0,
// 	"armor": 450,
// 	"stamina": 8,
// 	"power": 100,
// 	"codexSecret": false,
// 	"excludeFromCodex": true,
// 	"description": "Khora's will, Venari's fangs and claws.",
// 	"productCategory": "SpecialItems"
// },