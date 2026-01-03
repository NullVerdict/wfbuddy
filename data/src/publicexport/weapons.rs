#[derive(serde::Deserialize)]
pub struct Weapons {
	#[serde(rename = "ExportWeapons")]
	pub weapons: Vec<Weapon>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Weapon {
	pub unique_name: String,
	pub name: String,
}

// {
// 	"name": "Cedo Prime",
// 	"uniqueName": "/Lotus/Weapons/Tenno/LongGuns/PrimeCedo/PrimeCedoWeapon",
// 	"codexSecret": false,
// 	"damagePerShot": [
// 		0,
// 		32,
// 		0,
// 		0,
// 		0,
// 		0,
// 		0,
// 		0,
// 		0,
// 		0,
// 		0,
// 		0,
// 		0,
// 		0,
// 		0,
// 		0,
// 		0,
// 		0,
// 		0,
// 		0
// 	],
// 	"totalDamage": 32,
// 	"description": "A golden shotgun, forged for those who deliver judgment.",
// 	"criticalChance": 0.23999999,
// 	"criticalMultiplier": 2.4000001,
// 	"procChance": 0.019999981,
// 	"fireRate": 4.5,
// 	"masteryReq": 15,
// 	"productCategory": "LongGuns",
// 	"slot": 1,
// 	"accuracy": 20,
// 	"masteryReq": 15,
// 	"omegaAttenuation": 0.55000001,
// 	"noise": "ALARMING",
// 	"trigger": "AUTO",
// 	"magazineSize": 40,
// 	"reloadTime": 1.8,
// 	"multishot": 7
// },