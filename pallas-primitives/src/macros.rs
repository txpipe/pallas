#[macro_export]
macro_rules! create_struct_and_impls {
    ($struct_name:ident, $inner_type:ty, $tag:expr) => {
        #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
        pub struct $struct_name(Vec<$inner_type>);

        impl From<Vec<$inner_type>> for $struct_name {
            fn from(xs: Vec<$inner_type>) -> Self {
                $struct_name(xs)
            }
        }

        impl From<$struct_name> for Vec<$inner_type> {
            fn from(c: $struct_name) -> Self {
                c.0
            }
        }

        impl $struct_name {
            pub fn iter(&self) -> impl Iterator<Item = &$inner_type> {
                self.0.iter()
            }
        }

        impl <'b, C> minicbor::decode::Decode<'b, C> for $struct_name {
            fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
                if d.probe().tag().is_ok() {
                    d.tag()?;
                }
                Ok($struct_name(d.decode_with(ctx)?))
            }
        }

        impl <C> minicbor::encode::Encode<C> for $struct_name {
            fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
                if $tag {
                    e.tag(minicbor::data::Tag::Unassigned(258))?;
                }
                e.encode_with(&self.0, ctx)?;
                Ok(())
            }
        }
    };
}
