use common::item::ItemStack;

use super::components::{Container, HAlign, Padding, Text, VList, VPair};
use crate::{
    data::inventory::Inventory,
    input::Mouse,
    ui::{self, Element},
};
use glam::Vec4;
use std::{rc::Rc, ops::{Deref, DerefMut}};

#[derive(Clone, Debug)]
pub struct Recipe {
    pub ingredients: Vec<ItemStack>,
    pub outputs: Vec<ItemStack>,
}

impl Recipe {
    pub fn has_ingredients(&self, inventory: &Inventory) -> bool {
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

pub struct Button<'a, H: Handler> {
    component: Container<Padding<Text>>,
    mouse: &'a Mouse,
    on_click: H
}

pub trait Handler {
    fn handle(&mut self);
}

impl<'a, H: Handler> Button<'a, H> {
    pub fn new(mouse: &'a Mouse, text: &str, on_click: H) -> Self {
        Self {
            component: Container {
                child: Padding::new_uniform(
                    Text {
                        color: ui::color::get_highlight(),
                        content: text.to_owned(),
                    },
                    3,
                ),
                color: ui::color::get_background(),
                border_color: ui::color::get_highlight(),
                border_radius: 1,
            },
            mouse,
            on_click
        }
    }
}

impl<H: Handler> Element for Button<'_, H> {
    fn layout(&mut self, constraint: ui::SizeConstraints) -> glam::UVec2 {
        self.component.layout(constraint)
    }

    fn paint(&mut self, region: ui::Region, scene: &mut Vec<ui::Rectangle>) {
        if ui::input::hovering(self.mouse, &region) {
            self.component.color = Vec4::ONE;
        } else {
            self.component.color = ui::color::get_background();
        }

        if ui::input::clicked(self.mouse, &region, winit::event::MouseButton::Left) {
            self.on_click.handle()
        }

        self.component.paint(region, scene)
    }
}

pub struct CraftButtonHandler<'a> {
    recipe: Recipe,
    inventory: &'a mut Inventory
}

impl Handler for CraftButtonHandler<'_> {
    fn handle(&mut self) {
        if !self.recipe.has_ingredients(self.inventory) { return; }

        self.recipe.ingredients.iter().for_each(|stack| self.inventory.remove(*stack));
        self.recipe.outputs.iter().for_each(|stack| self.inventory.add(*stack));
    }
}

pub type Component<'a> = Container<Padding<VPair<VList<Text>, Button<'a, CraftButtonHandler<'a>>>>>;

impl<'a> Component<'a> {
    pub fn new(inventory: &'a mut Inventory, recipe: Recipe, mouse: &'a Mouse) -> Self {
        let mut text = Vec::new();
        let color = if recipe.has_ingredients(inventory) {
            ui::color::get_success()
        } else {
            ui::color::get_highlight()
        };

        text.push(Text {
            color,
            content: "Ingredients".to_owned(),
        });
        recipe.ingredients.iter().for_each(|ingredient| {
            let inventory_amount = inventory
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
        recipe.outputs.iter().for_each(|output| {
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

        let handler = CraftButtonHandler { recipe: recipe.clone(), inventory };
        let button = Button::new(mouse, "Craft", handler);

        let pair = VPair::new(text, button, HAlign::Center, 6);

        Self {
            child: Padding::new_uniform(pair, 2),
            color: ui::color::get_background(),
            border_color: ui::color::get_highlight(),
            border_radius: 1,
        }
    }
}
