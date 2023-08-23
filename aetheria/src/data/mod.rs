use common::item::ItemStack;

pub mod inventory;

#[derive(Clone, Debug)]
pub struct Recipe {
    pub ingredients: Vec<ItemStack>,
    pub outputs: Vec<ItemStack>,
}

impl Recipe {
    pub fn has_ingredients(&self, inventory: &inventory::Inventory) -> bool {
        self
            .ingredients
            .iter()
            .map(|ingredient| {
                ingredient.amount
                    <= inventory
                        .get_items()
                        .iter()
                        .find(|stack| stack.item == ingredient.item)
                        .map(|stack| stack.amount)
                        .unwrap_or(0)
            }).all(|x| x)
    }
}

pub struct Data {
    pub inventory: inventory::Inventory,
    pub current_recipe: Option<Recipe>,
    pub recipe_selections: Option<Vec<Recipe>>
}
