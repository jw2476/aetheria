use std::sync::{Arc, Mutex};
use super::components::{Container, Padding, VList, Button, Handler, HAlign};
use crate::{data::{Recipe, Data}, input::Mouse, ui};

pub type Component<'a> = Container<Padding<VList<Button<'a, RecipeSelectorHandler<'a>>>>>;

pub struct RecipeSelectorHandler<'a> {
    recipe: Recipe,
    data: Arc<Mutex<&'a mut Data>>
}

impl Handler for RecipeSelectorHandler<'_> {
    fn handle(&mut self) {
        self.data.lock().unwrap().current_recipe = Some(self.recipe.clone());
        self.data.lock().unwrap().recipe_selections = None;
    }
}

impl<'a> Component<'a> {
    pub fn new(data: &'a mut Data, mouse: &'a Mouse) -> Option<Self> {
        let recipes = data.recipe_selections.as_ref()?.clone();
        let data_mutex = Arc::new(Mutex::new(data));
        let buttons = recipes.iter().map(|recipe| {
            let handler = RecipeSelectorHandler { recipe: recipe.clone(), data: data_mutex.clone() };
            Button::new(mouse, &format!("{}", recipe.outputs[0]), handler)
        }).collect();

        Some(Self {
            child: Padding::new_uniform(VList { children: buttons, separation: 2, align: HAlign::Left }, 2),
            color: ui::color::get_background(),
            border_radius: 1,
            border_color: ui::color::get_highlight()
        })
    }
}
