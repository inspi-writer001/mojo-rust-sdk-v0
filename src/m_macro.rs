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

    #[test]
    fn mojo_enumish() {
        // since we can't derive Pod on Enums... users may need to write more code to make their enum fit.. and probably some padding
        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        pub struct FlyAbility(u8);

        impl FlyAbility {
            pub const CAN_FLY: Self = Self(0);
            pub const CAN_NOT_FLY: Self = Self(1);
        }

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
