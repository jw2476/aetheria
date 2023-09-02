use common::item::ItemStack;

use super::components::{
    Button, Container, HAlign, HPair, Handler, Padding, Text, VAlign, VList, VPair,
};
use crate::{
    data::{inventory::Inventory, Data, Recipe},
    input::Mouse,
    ui::{self, Element},
};
use glam::Vec4;
use std::sync::{Arc, Mutex};

pub struct CraftButtonHandler<'a> {
    recipe: Recipe,
    data: Arc<Mutex<&'a mut Data>>,
}

impl Handler for CraftButtonHandler<'_> {
    fn handle(&mut self) {
        if !self
            .recipe
            .has_ingredients(&self.data.lock().unwrap().inventory)
        {
            return;
        }

        self.recipe
            .ingredients
            .iter()
            .for_each(|stack| self.data.lock().unwrap().inventory.remove(*stack));
        self.recipe
            .outputs
            .iter()
            .for_each(|stack| self.data.lock().unwrap().inventory.add(*stack));

        self.data.lock().unwrap().current_recipe = None;
    }
}

pub struct CloseHandler<'a> {
    data: Arc<Mutex<&'a mut Data>>,
}

impl Handler for CloseHandler<'_> {
    fn handle(&mut self) {
        self.data.lock().unwrap().current_recipe = None
    }
}

pub type Component<'a> = Container<
    Padding<
        VPair<VList<Text>, HPair<Button<'a, CloseHandler<'a>>, Button<'a, CraftButtonHandler<'a>>>>,
    >,
>;

impl<'a> Component<'a> {
    pub fn new(data: &'a mut Data, mouse: &'a Mouse) -> Option<Self> {
        let mut text = Vec::new();
        let color = if data
            .current_recipe
            .as_ref()?
            .has_ingredients(&data.inventory)
        {
            ui::color::get_success()
        } else {
            ui::color::get_highlight()
        };

        text.push(Text {
            color,
            content: "Ingredients".to_owned(),
        });
        data.current_recipe
            .as_ref()?
            .ingredients
            .iter()
            .for_each(|ingredient| {
                let inventory_amount = data
                    .inventory
                    .get_items()
                    .iter()
                    .find(|stack| stack.item == ingredient.item)
                    .map(|stack| stack.amount)
                    .unwrap_or(0);

                let color = if inventory_amount >= ingredient.amount {
                    ui::color::get_success()
                } else {
                    ui::color::get_highlight()
                };

                text.push(Text {
                    color,
                    content: format!(
                        "{} {}/{}",
                        ingredient.item, inventory_amount, ingredient.amount
                    ),
                })
            });
        text.push(Text {
            color: Vec4::ZERO,
            content: String::new(),
        });
        text.push(Text {
            color: ui::color::get_highlight(),
            content: "Outputs".to_owned(),
        });
        data.current_recipe
            .as_ref()?
            .outputs
            .iter()
            .for_each(|output| {
                text.push(Text {
                    color: ui::color::get_highlight(),
                    content: format!("{}", output),
                })
            });

        let text = VList {
            children: text,
            separation: 2,
            align: HAlign::Left,
        };

        let recipe = data.current_recipe.clone()?;
        let data_mutex = Arc::new(Mutex::new(data));
        let craft_handler = CraftButtonHandler {
            recipe,
            data: data_mutex.clone(),
        };
        let craft_button = Button::new(mouse, "Craft", craft_handler);
        let close_handler = CloseHandler { data: data_mutex };
        let close_button = Button::new(mouse, "Cancel", close_handler);

        let pair = VPair::new(
            text,
            HPair::new(close_button, craft_button, VAlign::Top, 4),
            HAlign::Center,
            6,
        );

        Some(Self {
            child: Padding::new_uniform(pair, 2),
            color: ui::color::get_background(),
            border_color: ui::color::get_highlight(),
            border_radius: 1,
        })
    }
}
