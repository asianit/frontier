// SPDX-License-Identifier: Apache-2.0
// This file is part of Frontier.
//
// Copyright (c) 2020 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Test mock for unit tests and benchmarking

use frame_support::{
	parameter_types,
	traits::{ConstU32, Currency, FindAuthor, Imbalance, OnUnbalanced},
	ConsensusEngineId,
};
use frame_support::metadata::StorageEntryModifier::Default;
use sp_core::{H160, H256, U256};
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentityLookup},
};
use sp_std::{boxed::Box, prelude::*, str::FromStr};

use crate::{
	AddressMapping, EVMCurrencyAdapter, EnsureAddressNever, EnsureAddressRoot, FeeCalculator,
	IdentityAddressMapping,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime! {
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage},
		EVM: crate::{Pallet, Call, Storage, Config, Event<T>},
	}
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
}
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Call = Call;
	type Hashing = BlakeTwo256;
	type AccountId = H160;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = generic::Header<u64, BlakeTwo256>;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 0;
}
impl pallet_balances::Config for Test {
	type MaxLocks = ();
	type Balance = u64;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxReserves = ();
	type ReserveIdentifier = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = 1000;
}
impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

/// Fixed gas price of `0`.
pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> U256 {
		1_000_000_000u128.into()
	}
}

pub struct FindAuthorTruncated;
impl FindAuthor<H160> for FindAuthorTruncated {
	fn find_author<'a, I>(_digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		Some(H160::from_str("1234500000000000000000000000000000000000").unwrap())
	}
}

type NegativeImbalance = <Balances as Currency<H160>>::NegativeImbalance;

impl OnUnbalanced<NegativeImbalance> for FindAuthorTruncated {
	fn on_nonzero_unbalanced(amount: NegativeImbalance) {
		let author = H160::from_str("1234500000000000000000000000000000000000").unwrap();
		let author_substrate = <Test as crate::Config>::AddressMapping::into_account_id(author);
		Balances::resolve_creating(&author_substrate, amount);
	}
}

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
		if let Some(_fees) = fees_then_tips.next() {
			// 1. for fee, default to be burned.

			// 2. for tips, if any, 100% to author.
			if let Some(tips) = fees_then_tips.next() {
				FindAuthorTruncated::on_unbalanced(tips);
			}
		}
	}
}

impl crate::Config for Test {
	type FeeCalculator = FixedGasPrice;
	type GasWeightMapping = ();

	type CallOrigin = EnsureAddressRoot<Self::AccountId>;
	type WithdrawOrigin = EnsureAddressNever<Self::AccountId>;

	type AddressMapping = IdentityAddressMapping;
	type Currency = Balances;
	type Runner = crate::runner::stack::Runner<Self>;

	type Event = Event;
	type PrecompilesType = ();
	type PrecompilesValue = ();
	type ChainId = ();
	type BlockGasLimit = ();
	type OnChargeTransaction = EVMCurrencyAdapter<Balances, DealWithFees>;
	type BlockHashMapping = crate::SubstrateBlockHashMapping<Self>;
	type FindAuthor = FindAuthorTruncated;

	type WeightInfo = ();
}
