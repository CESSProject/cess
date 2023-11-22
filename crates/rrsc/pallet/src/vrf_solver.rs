use frame_election_provider_support::{Assignment, NposSolver, WeightInfo as NposWeightInfo};
use frame_support::{
	traits::{Get, Randomness},
	weights::Weight
};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};
use sp_npos_elections::{
	ElectionResult, ExtendedBalance, IdentifierT, PerThing128, VoteWeight,
};
use super::{Config, ParentBlockRandomness, EpochIndex};
use codec::{alloc::string::ToString, Decode};
use cessp_consensus_rrsc::traits::ValidatorCredits;

pub trait VrfSloverConfig {

	/// A target whose vote weight is less than `min_electable_weight` will never be elected.
	fn min_electable_weight() -> VoteWeight;
}
/// A wrapper for elect by vrf that implements [`NposSolver`].
pub struct VrfSolver<AccountId, Accuracy, T, Credits, SloverConfig, Balancing = ()>(
	sp_std::marker::PhantomData<(AccountId, Accuracy, T, Credits, SloverConfig, Balancing)>,
);

impl<
		AccountId: IdentifierT,
		Accuracy: PerThing128,
		T: Config,
		Credits: ValidatorCredits<AccountId>,
		SloverConfig: VrfSloverConfig,
		Balancing: Get<Option<(usize, ExtendedBalance)>>,
	> NposSolver for VrfSolver<AccountId, Accuracy, T, Credits, SloverConfig, Balancing>
{
	type AccountId = AccountId;
	type Accuracy = Accuracy;
	type Error = sp_npos_elections::Error;
	fn solve(
		winners: usize,
		targets: Vec<Self::AccountId>,
		voters: Vec<(Self::AccountId, VoteWeight, impl IntoIterator<Item = Self::AccountId>)>,
	) -> Result<ElectionResult<Self::AccountId, Self::Accuracy>, Self::Error> {
		let to_elect = winners;

		let ElectionResult { winners, assignments } = Self::setup_inputs(targets, voters);
		// max_stake is used as a benchmark value of 100 stake_score
		let mut max_stake: ExtendedBalance = SloverConfig::min_electable_weight().into();
		winners.clone().into_iter().for_each(|(_, backed_stake)| {
			if backed_stake > max_stake {
				max_stake = backed_stake;
			}
		});

		let credits = Credits::credits(EpochIndex::<T>::get());
		let full_credit = Credits::full_credit();
		
		let mut account_scores = winners
			.into_iter()
			.enumerate()
			.map(|(account_index, (account_id, backed_stake))| {
				// credit_score
				let credit_score = match credits.get(&account_id) {
					Some(c) => *c,
					None => 0,
				};
				// stake_score
				let stake_score = Accuracy::from_rational(backed_stake, max_stake).mul_floor(100) as u32;
				// random_score
				let random_number = Self::random_number("authorities", &account_index);
				let random_score = random_number % full_credit;
				// final_score = `credit_score` * 50% + `stake_score` * 30% + `random_score` * 20%
				let final_score = credit_score.saturating_mul(5)
							.saturating_add(stake_score.saturating_mul(3))
							.saturating_add(random_score.saturating_mul(2))
							.saturating_div(10);
				
				log::debug!(
					target: "rrsc::vrf_solver",
					"account: {:?}, credit_score: {:?}, stake_score: {:?}, random_score: {:?}, final_score: {:?}",
					account_id,
					credit_score,
					stake_score,
					random_score,
					final_score,
				);
				(account_id, backed_stake, final_score)
			})
			.collect::<Vec<(AccountId, ExtendedBalance, u32)>>();

		account_scores.sort_by_key(|e| e.2);
		account_scores.reverse();

		let winners = account_scores
			.into_iter()
			.take(to_elect)
			.map(|e| (e.0, e.1))
			.collect::<Vec<_>>();
		let winner_accounts = winners
			.clone()
			.into_iter()
			.map(|w| w.0)
			.collect::<Vec<AccountId>>();
		
		let assignments = assignments
			.into_iter()
			.filter_map(|assignment| {
				let mut distribution: Vec<(AccountId, Accuracy)> = Vec::new();
				for d in assignment.distribution {
					if winner_accounts.contains(&d.0) {
						distribution.push(d);
					} // else {} would be wrong votes. We don't really care about it.
				}
				if distribution.is_empty() {
					None
				} else {
					Some(Assignment { who: assignment.who, distribution })
				}
			})
			.collect::<Vec<_>>();
		
		log::debug!(target: "rrsc::vrf_solver", "[solve] winners: {:#?}", winners);
		log::debug!(target: "rrsc::vrf_solver", "[solve] assignments: {:#?}", assignments);
		Ok(ElectionResult { winners, assignments })
	}

	fn weight<W: NposWeightInfo>(voters: u32, targets: u32, vote_degree: u32) -> Weight {
		W::phragmen(voters, targets, vote_degree)
	}
}

impl <
AccountId: IdentifierT,
Accuracy: PerThing128,
T: Config,
Credits: ValidatorCredits<AccountId>,
SloverConfig: VrfSloverConfig,
Balancing: Get<Option<(usize, ExtendedBalance)>>,
> VrfSolver<AccountId, Accuracy, T, Credits, SloverConfig, Balancing> {
	pub fn random_number(context: &str,authority_index: &usize) -> u32 {
		let mut b_context = context.to_string();
		b_context.push_str(authority_index.to_string().as_str());
		let (hash, _) = ParentBlockRandomness::<T>::random(&b_context.as_bytes());
		let hash = 	match hash {
				Some(h) => h,
				None => T::Hash::default(),
		};
		log::debug!(target: "rrsc::vrf_solver", "{:?} Hash: {:?}", b_context, hash);
		let random_number = u32::decode(&mut hash.as_ref())
								.expect("secure hashes should always be bigger than u32; qed");
		random_number
	}

	/// Converts raw inputs to types used in this crate.
	///
	/// This will perform some cleanup that are most often important:
	/// - It drops any votes that are pointing to non-candidates.
	/// - It drops duplicate targets within a voter.
	fn setup_inputs(
		initial_candidates: Vec<AccountId>,
		initial_voters: Vec<(AccountId, VoteWeight, impl IntoIterator<Item = AccountId>)>,
	) -> ElectionResult<AccountId, Accuracy> {
		let mut candidates = BTreeMap::<AccountId, ExtendedBalance>::new();

		initial_candidates
			.into_iter()
			.for_each(|who| {
				candidates.insert(who, 0u128);
			});

		let assignments = initial_voters
			.into_iter()
			.filter_map(|(who, vote_weight, votes)| {
				let mut distribution: Vec<(AccountId, Accuracy)> = Vec::new();
				let votes_vec = votes.into_iter().collect::<Vec<_>>();
				log::debug!(target: "rrsc::vrf_solver", "[setup_inputs] vote: {:#?}, {:#?}, {:#?}", who, vote_weight, votes_vec);
				let votes_count = votes_vec.len() as u128;
				for v in votes_vec {
					if distribution.iter().any(|e| e.0 == v) {
						// duplicate vote.
						continue
					}
					if let Some(backed_stake) = candidates.get_mut(&v) {
						// This candidate is valid.
						*backed_stake = backed_stake.saturating_add((vote_weight as u128).saturating_div(votes_count));
						distribution.push((v.clone(), Accuracy::from_rational(1, votes_count)));
					} // else {} would be wrong votes. We don't really care about it.
				}
				if distribution.is_empty() {
					None
				} else {
					Some(Assignment { who, distribution })
				}
			})
			.collect::<Vec<_>>();

		log::debug!(target: "rrsc::vrf_solver", "[setup_inputs] candidates: {:#?}", candidates);
		let winners = candidates
			.into_iter()
			.filter(|(_, bakced_stake)| *bakced_stake >= SloverConfig::min_electable_weight().into())
			.collect::<Vec<_>>();
		log::debug!(target: "rrsc::vrf_solver", "[setup_inputs] winners: {:#?}", winners);
		log::debug!(target: "rrsc::vrf_solver", "[setup_inputs] assignments: {:#?}", assignments);

		ElectionResult { winners, assignments }
	}
}
