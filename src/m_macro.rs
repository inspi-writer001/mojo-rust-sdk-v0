#[macro_export]
macro_rules! mojo {
    ($visibility:vis struct $struct_name:ident {$($member_vis:vis $member_value:ident : $member_type:ty),* $(,)?}) => {

        #[repr(C)]
        #[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
        $visibility struct $struct_name {
            $($member_vis $member_value : $member_type),*
        }

        impl $struct_name {
            pub const LEN: usize = core::mem::size_of::<Self>();

            pub fn to_bytes(&self) -> &[u8] {
                bytemuck::bytes_of(self)
            }

            pub fn len(&self) -> usize {
                Self::LEN
            }
        }


    };
}

#[macro_export]
macro_rules! mojo_enum {
    // Usage: mojo_enum! { pub enum Name : u8 { Variant = 0, ... } }
    ($vis:vis enum $name:ident : $backing_type:ty {
        $($variant:ident = $val:expr),* $(,)?
    }) => {
        // 1. Create the wrapper struct
        // repr(transparent) ensures it has the exact same layout as the backing type (e.g., u8)
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, bytemuck::Pod, bytemuck::Zeroable)]
        $vis struct $name($backing_type);

        // 2. Create the constants inside the struct namespace
        impl $name {
            $(
                pub const $variant: Self = Self($val);
            )*
        }
    };
}

#[cfg(test)]
mod test_macro {

    #[test]
    fn mojo_preferred() {
        mojo! {
            pub struct Person {
                pub height: [u8; 8],
                pub length: [u8; 8],
                pub can_fly: [u8; 1],
            }
        }

        let new_guy = Person {
            height: 259u64.to_le_bytes(),
            length: 5674u64.to_le_bytes(),
            can_fly: 0u8.to_le_bytes(),
        };

        println!("Bytes: {:?}", new_guy.to_bytes());
        println!("Size: {}", new_guy.len());

        assert_eq!(
            new_guy.to_bytes(),
            [3, 1, 0, 0, 0, 0, 0, 0, 42, 22, 0, 0, 0, 0, 0, 0, 0],
            "Values aren't equal"
        );

        assert_eq!(new_guy.len(), 17, "Length aren't the same");
    }

    #[allow(unused)]
    #[test]
    fn mojo_enumish() {
        mojo_enum! { pub enum FlyAbility: u8 {
            CAN_FLY = 0,
            CANNOT_FLY = 1,
        }}

        mojo! {
            pub struct Person {
                pub height: u64,
                pub length: u64,
                pub can_fly: FlyAbility,
                _padding: [u8; 7]
            }
        }

        let person = Person {
            height: 180,
            length: 75,
            can_fly: FlyAbility::CAN_FLY,
            _padding: [0; 7],
        };

        println!("Bytes: {:?}", person.to_bytes());
        println!("Size: {}", Person::LEN);

        assert_eq!(
            person.to_bytes(),
            [180, 0, 0, 0, 0, 0, 0, 0, 75, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            "Values aren't same"
        );

        assert_eq!(person.len(), 24, "Length aren't the same");
    }
}
