#[derive(serde::Deserialize)]
pub struct Recipes {
	#[serde(rename = "ExportRecipes")]
	pub recipes: Vec<Recipe>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
	pub unique_name: String,
	pub result_type: String,
	// pub ingredients: Vec<Ingredient>,
}

// #[derive(serde::Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct Ingredient {
// 	pub item_type: String,
// 	pub item_count: i32,
// }

// {
// 	"uniqueName": "/Lotus/Types/Recipes/WarframeRecipes/ZephyrPrimeChassisBlueprint",
// 	"resultType": "/Lotus/Types/Recipes/WarframeRecipes/ZephyrPrimeChassisComponent",
// 	"buildPrice": 15000,
// 	"buildTime": 43200,
// 	"skipBuildTimePrice": 25,
// 	"consumeOnUse": true,
// 	"num": 1,
// 	"codexSecret": false,
// 	"primeSellingPrice": 25,
// 	"ingredients": [
// 		{
// 			"ItemType": "/Lotus/Types/Items/MiscItems/Alertium",
// 			"ItemCount": 2,
// 			"ProductCategory": "MiscItems"
// 		},
// 		{
// 			"ItemType": "/Lotus/Types/Items/MiscItems/Tellurium",
// 			"ItemCount": 2,
// 			"ProductCategory": "MiscItems"
// 		},
// 		{
// 			"ItemType": "/Lotus/Types/Items/MiscItems/Ferrite",
// 			"ItemCount": 3600,
// 			"ProductCategory": "MiscItems"
// 		},
// 		{
// 			"ItemType": "/Lotus/Types/Items/MiscItems/OxiumAlloy",
// 			"ItemCount": 300,
// 			"ProductCategory": "MiscItems"
// 		}
// 	],
// 	"secretIngredients": []
// },