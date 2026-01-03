#[derive(serde::Deserialize)]
pub struct Warframes {
	#[serde(rename = "ExportWarframes")]
	pub warframes: Vec<Warframe>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Warframe {
	pub unique_name: String,
	pub name: String,
}

// {
// 	"uniqueName": "/Lotus/Powersuits/Tengu/ZephyrPrime",
// 	"name": "Zephyr Prime",
// 	"parentName": "/Lotus/Powersuits/Tengu/TenguBaseSuit",
// 	"description": "Take to the skies with this golden bird of destruction.",
// 	"health": 455,
// 	"shield": 455,
// 	"armor": 135,
// 	"stamina": 3,
// 	"power": 175,
// 	"codexSecret": false,
// 	"masteryReq": 0,
// 	"sprintSpeed": 1.15,
// 	"passiveDescription": "Zephyr moves faster and falls slower while airborne. Also gain |CRIT|% Critical Hit chance with weapons while airborne.",
// 	"abilities": [
// 		{
// 			"abilityUniqueName": "/Lotus/Powersuits/PowersuitAbilities/TailWindAbility",
// 			"abilityName": "Tail Wind",
// 			"description": "Hold while airborne to hover Zephyr with reduced movement. From the air, tap to dash forward, or aim down to dive bomb enemies below."
// 		},
// 		{
// 			"abilityUniqueName": "/Lotus/Powersuits/PowersuitAbilities/TenguBurstAbility",
// 			"abilityName": "Airburst",
// 			"description": "Launch a burst of massively dense air. Hold to send enemies flying, tap to pull them toward the burst. Damage increases per enemy hit."
// 		},
// 		{
// 			"abilityUniqueName": "/Lotus/Powersuits/PowersuitAbilities/TurbulenceAbility",
// 			"abilityName": "Turbulence",
// 			"description": "Creates a wind shield around Zephyr, redirecting all incoming projectiles."
// 		},
// 		{
// 			"abilityUniqueName": "/Lotus/Powersuits/PowersuitAbilities/TornadoAbility",
// 			"abilityName": "Tornado",
// 			"description": "Create deadly tornadoes that seek out and engulf enemies. Tornadoes deal the elemental Damage Type they absorb the most. Shoot engulfed enemies to inflict extra damage. Hold for stationary tornadoes or tap for wandering ones."
// 		}
// 	],
// 	"productCategory": "Suits"
// },