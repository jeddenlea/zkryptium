// Copyright 2023 Fondazione LINKS

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use bls12_381_plus::G1Projective;
use elliptic_curve::group::Curve;
use elliptic_curve::hash2curve::{ExpandMsg, Expander};
use serde::{Serialize, Deserialize};
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use serde::ser::{Serializer, SerializeStruct};
use crate::bbsplus::keys::BBSplusPublicKey;
use super::ciphersuites::BbsCiphersuite;




#[derive(Clone, PartialEq, Eq, Debug, Deserialize)]
pub struct Generators {
    pub g1_base_point: G1Projective,
    pub q1: G1Projective,
    pub q2: G1Projective,
    pub message_generators: Vec<G1Projective>
}

impl Serialize for Generators {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let result: Vec<String> = self.message_generators.iter()
            .map(|item| hex::encode(item.to_affine().to_compressed())).collect();

        let mut state = serializer.serialize_struct("Generators", 4)?;
        state.serialize_field("BP",
            &hex::encode(self.g1_base_point.to_affine().to_compressed()))?;

        state.serialize_field("Q1",
            &hex::encode(self.q1.to_affine().to_compressed()))?;
        state.serialize_field("Q2", 
            &hex::encode(self.q2.to_affine().to_compressed()))?;

        state.serialize_field("MsgGenerators", &result)?;
        state.end()
    }
}



pub fn global_generators<F>(make_generators_fn: F, len: usize) -> Generators
where
    F: for<'r> Fn(Option<&'r [u8]>, usize) -> Generators
{
    let mut len = len;
    if len < 2 {
        println!("len must be at least 2 -> default set to 2");
        len = 2;
    }
    make_generators_fn(None, len)
}

pub fn signer_specific_generators<F>(pk: &BBSplusPublicKey, make_generators_fn: F, len: usize) -> Generators
where
    F: for<'r> Fn(Option<&'r [u8]>, usize) -> Generators
{

    // let mut rng = rand::thread_rng();
    // let kp = KeyPair::<BBSplus<Bls12381Sha256>>::generate_rng(&mut rng);

    let mut len = len;
    if len < 2 {
        println!("len must be at least 2 -> default set to 2");
        len = 2;
    }

    make_generators_fn(Some(&pk.to_bytes()), len)
}

pub fn print_generators(generators: &Generators) {
    println!("G1 BP = {}", hex::encode(
        generators.g1_base_point.to_affine().to_compressed()
    ));

    println!("Q_1 = {}", hex::encode(
        generators.q1.to_affine().to_compressed()
    ));

    println!("Q_2 = {}", hex::encode(
        generators.q2.to_affine().to_compressed()
    ));
    
    generators.message_generators.iter().enumerate().for_each(|(i, g)| {
        println!(
            "G_{} = {}",
            i + 1,
            hex::encode(g.to_affine().to_compressed())
        );
    });
}

pub fn write_generators_to_file(generators: &Generators, file_name: String) {
    let path = env::current_dir().unwrap();

    let file_path = path.join(file_name);

    let file = File::create(file_path).unwrap();

    let mut writer = BufWriter::new(file);

    serde_json::to_writer_pretty(&mut writer, &generators).unwrap();

    writer.flush().unwrap();
}

pub fn make_generators<X>(seed: Option<&[u8]>, len: usize) -> Generators
where
    X: BbsCiphersuite,
    X::Expander: for<'a> ExpandMsg<'a>,
{
    let default_seed = &X::GENERATOR_SEED;
    let seed = seed.unwrap_or(default_seed);

    let base_point = make_g1_base_point::<X>();
    let mut generators = Vec::new();

    let mut v = vec!(0u8; X::EXPAND_LEN);
    let mut buffer = vec!(0u8; X::EXPAND_LEN);

    X::Expander::expand_message(&[seed], &[X::GENERATOR_SEED_DST], X::EXPAND_LEN).unwrap().fill_bytes(&mut v);
    let mut n = 1u32;
    while generators.len() < len {
        v.append(n.to_be_bytes().to_vec().as_mut());
        X::Expander::expand_message(&[&v], &[X::GENERATOR_SEED_DST], X::EXPAND_LEN).unwrap().fill_bytes(&mut buffer);
        v = buffer.clone();
        n += 1;
        let candidate = G1Projective::hash::<X::Expander>(&v, &X::GENERATOR_DST);
        if !generators.contains(&candidate) {
            generators.push(candidate);
        }
    }

    Generators {
        g1_base_point: base_point,
        q1: generators[0].clone(),
        q2: generators[1].clone(),
        message_generators: generators[2..].to_vec()
    }
}

pub fn make_g1_base_point<X>() -> G1Projective
where
    X: BbsCiphersuite,
    X::Expander: for<'a> ExpandMsg<'a>,
{

    let mut v = [0u8; 48];
    X::Expander::expand_message(&[X::GENERATOR_SEED_BP], &[X::GENERATOR_SEED_DST], X::EXPAND_LEN).unwrap().fill_bytes(&mut v);

    // TODO: implement a proper I2OSP
    let extra = 1u32.to_be_bytes().to_vec();
    let buffer = [v.as_ref(), &extra].concat();

    X::Expander::expand_message(&[&buffer], &[X::GENERATOR_SEED_DST], X::EXPAND_LEN).unwrap().fill_bytes(&mut v);

    G1Projective::hash::<X::Expander>(&v, &X::GENERATOR_DST)
}
