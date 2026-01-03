#[derive(serde::Deserialize)]
pub struct Resources {
	#[serde(rename = "ExportResources")]
	pub resources: Vec<Resource>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
	pub unique_name: String,
	pub name: String,
}

// {
// 	"uniqueName": "/Lotus/Types/Recipes/WarframeRecipes/ZephyrPrimeChassisComponent",
// 	"name": "Zephyr Prime Chassis",
// 	"description": "Chassis component of the Zephyr Prime Warframe.",
// 	"codexSecret": false,
// 	"parentName": "/Lotus/Types/Items/MiscItems/WarframeComponentItem",
// 	"primeSellingPrice": 25
// },