
pub mod v01;

use crate::*;
use codec::{Codec, Decode};
use core::marker::PhantomData;
use frame_support::{
	pallet_prelude::*,
	traits::{ConstU32, OnRuntimeUpgrade},
	weights::WeightMeter,
};
use sp_runtime::Saturating;

#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

const PROOF_ENCODE: &str = "Tuple::max_encoded_len() < Cursor::max_encoded_len()` is verified in `Self::integrity_test()`; qed";
const PROOF_DECODE: &str =
	"We encode to the same type in this trait only. No other code touches this item; qed";

fn invalid_version(version: StorageVersion) -> ! {
	panic!("Required migration {version:?} not supported by this runtime. This is a bug.");
}

/// The cursor used to encode the position (usually the last iterated key) of the current migration
/// step.
pub type Cursor = BoundedVec<u8, ConstU32<1024>>;

/// IsFinished describes whether a migration is finished or not.
pub enum IsFinished {
	Yes,
	No,
}

/// A trait that allows to migrate storage from one version to another.
///
/// The migration is done in steps. The migration is finished when
/// `step()` returns `IsFinished::Yes`.
pub trait MigrationStep: Codec + MaxEncodedLen + Default {
	/// Returns the version of the migration.
	const VERSION: u16;

	/// Returns the maximum weight that can be consumed in a single step.
	fn max_step_weight() -> Weight;

	/// Process one step of the migration.
	///
	/// Returns whether the migration is finished.
	fn step(&mut self, meter: &mut WeightMeter) -> IsFinished;

	/// Verify that the migration step fits into `Cursor`, and that `max_step_weight` is not greater
	/// than `max_block_weight`.
	fn integrity_test(max_block_weight: Weight) {
		if Self::max_step_weight().any_gt(max_block_weight) {
			panic!(
				"Invalid max_step_weight for Migration {}. Value should be lower than {}",
				Self::VERSION,
				max_block_weight
			);
		}

		let len = <Self as MaxEncodedLen>::max_encoded_len();
		let max = Cursor::bound();
		if len > max {
			panic!(
				"Migration {} has size {} which is bigger than the maximum of {}",
				Self::VERSION,
				len,
				max,
			);
		}
	}

	/// Execute some pre-checks prior to running the first step of this migration.
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade_step() -> Result<Vec<u8>, TryRuntimeError> {
		Ok(Vec::new())
	}

	/// Execute some post-checks after running the last step of this migration.
	#[cfg(feature = "try-runtime")]
	fn post_upgrade_step(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
		Ok(())
	}
}

/// A noop migration that can be used when there is no migration to be done for a given version.
#[doc(hidden)]
#[derive(frame_support::DefaultNoBound, Encode, Decode, MaxEncodedLen)]
pub struct NoopMigration<const N: u16>;

impl<const N: u16> MigrationStep for NoopMigration<N> {
	const VERSION: u16 = N;
	fn max_step_weight() -> Weight {
		Weight::zero()
	}
	fn step(&mut self, _meter: &mut WeightMeter) -> IsFinished {
		IsFinished::Yes
	}
}

mod private {
	use crate::migration::MigrationStep;
	pub trait Sealed {}
	#[impl_trait_for_tuples::impl_for_tuples(10)]
	#[tuple_types_custom_trait_bound(MigrationStep)]
	impl Sealed for Tuple {}
}

/// Defines a sequence of migrations.
///
/// The sequence must be defined by a tuple of migrations, each of which must implement the
/// `MigrationStep` trait. Migrations must be ordered by their versions with no gaps.
pub trait MigrateSequence: private::Sealed {
	/// Returns the range of versions that this migrations sequence can handle.
	/// Migrations must be ordered by their versions with no gaps.
	///
	/// The following code will fail to compile:
	///
	/// ```compile_fail
	///     # use pallet_contracts::{NoopMigration, MigrateSequence};
	/// 	let _ = <(NoopMigration<1>, NoopMigration<3>)>::VERSION_RANGE;
	/// ```
	/// The following code will compile:
	/// ```
	///     # use pallet_contracts::{NoopMigration, MigrateSequence};
	/// 	let _ = <(NoopMigration<1>, NoopMigration<2>)>::VERSION_RANGE;
	/// ```
	const VERSION_RANGE: (u16, u16);

	/// Returns the default cursor for the given version.
	fn new(version: StorageVersion) -> Cursor;

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade_step(_version: StorageVersion) -> Result<Vec<u8>, TryRuntimeError> {
		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade_step(_version: StorageVersion, _state: Vec<u8>) -> Result<(), TryRuntimeError> {
		Ok(())
	}

	/// Execute the migration step until the available weight is consumed.
	fn steps(version: StorageVersion, cursor: &[u8], meter: &mut WeightMeter) -> StepResult;

	/// Verify that the migration step fits into `Cursor`, and that `max_step_weight` is not greater
	/// than `max_block_weight`.
	fn integrity_test(max_block_weight: Weight);

	/// Returns whether migrating from `in_storage` to `target` is supported.
	///
	/// A migration is supported if `VERSION_RANGE` is (in_storage + 1, target).
	fn is_upgrade_supported(in_storage: StorageVersion, target: StorageVersion) -> bool {
		let (low, high) = Self::VERSION_RANGE;
		target == high && in_storage + 1 == low
	}
}

/// Performs all necessary migrations based on `StorageVersion`.
///
/// If `TEST_ALL_STEPS == true` and `try-runtime` is enabled, this will run all the migrations
/// inside `on_runtime_upgrade`. This should be set to false in tests that want to ensure the step
/// by step migration works.
pub struct Migration<T: Config, const TEST_ALL_STEPS: bool = true>(PhantomData<T>);

#[cfg(feature = "try-runtime")]
impl<T: Config, const TEST_ALL_STEPS: bool> Migration<T, TEST_ALL_STEPS> {
	fn run_all_steps() -> Result<(), TryRuntimeError> {
		let mut meter = &mut WeightMeter::new();
		let name = <Pallet<T>>::name();
		loop {
			let in_progress_version = <Pallet<T>>::on_chain_storage_version() + 1;
			let state = T::Migrations::pre_upgrade_step(in_progress_version)?;
			let before = meter.consumed();
			let status = Self::migrate(&mut meter);
			T::Migrations::post_upgrade_step(in_progress_version, state)?;
			if matches!(status, MigrateResult::Completed) {
				break
			}
		}

		let name = <Pallet<T>>::name();
		Ok(())
	}
}

impl<T: Config, const TEST_ALL_STEPS: bool> OnRuntimeUpgrade for Migration<T, TEST_ALL_STEPS> {
	fn on_runtime_upgrade() -> Weight {
		let name = <Pallet<T>>::name();
		let in_code_version = <Pallet<T>>::in_code_storage_version();
		let on_chain_version = <Pallet<T>>::on_chain_storage_version();

		if on_chain_version == in_code_version {
			return <T as pallet::Config>::WeightInfo::on_runtime_upgrade_noop()
		}

		// In case a migration is already in progress we create the next migration
		// (if any) right when the current one finishes.
		if Self::in_progress() {
			return <T as pallet::Config>::WeightInfo::on_runtime_upgrade_in_progress()
		}


		let cursor = T::Migrations::new(on_chain_version + 1);
		MigrationInProgress::<T>::set(Some(cursor));

		#[cfg(feature = "try-runtime")]
		if TEST_ALL_STEPS {
			Self::run_all_steps().unwrap();
		}

		<T as pallet::Config>::WeightInfo::on_runtime_upgrade()
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		// We can't really do much here as our migrations do not happen during the runtime upgrade.
		// Instead, we call the migrations `pre_upgrade` and `post_upgrade` hooks when we iterate
		// over our migrations.
		let on_chain_version = <Pallet<T>>::on_chain_storage_version();
		let in_code_version = <Pallet<T>>::in_code_storage_version();

		if on_chain_version == in_code_version {
			return Ok(Default::default())
		}

		ensure!(
			T::Migrations::is_upgrade_supported(on_chain_version, in_code_version),
			"Unsupported upgrade: VERSION_RANGE should be (on-chain storage version + 1, in-code storage version)"
		);

		Ok(Default::default())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
		if !TEST_ALL_STEPS {
			return Ok(())
		}

		// Ensure that the hashing algorithm is correct for each storage map.
		if let Some(hash) = crate::MinerItems::<T>::iter_keys().next() {
			crate::MinerItems::<T>::get(hash).expect("CodeInfo exists for hash; qed");
		}

		Ok(())
	}
}

/// The result of running the migration.
#[derive(Debug, PartialEq)]
pub enum MigrateResult {
	/// No migration was performed
	NoMigrationPerformed,
	/// No migration currently in progress
	NoMigrationInProgress,
	/// A migration is in progress
	InProgress { steps_done: u32 },
	/// All migrations are completed
	Completed,
}

/// The result of running a migration step.
#[derive(Debug, PartialEq)]
pub enum StepResult {
	InProgress { cursor: Cursor, steps_done: u32 },
	Completed { steps_done: u32 },
}

impl<T: Config, const TEST_ALL_STEPS: bool> Migration<T, TEST_ALL_STEPS> {
	/// Verify that each migration's step of the [`Config::Migrations`] sequence fits into
	/// `Cursor`.
	pub(crate) fn integrity_test() {
		let max_weight = <T as frame_system::Config>::BlockWeights::get().max_block;
		T::Migrations::integrity_test(max_weight)
	}

	/// Execute the multi-step migration.
	/// Returns whether or not a migration is in progress
	pub(crate) fn migrate(mut meter: &mut WeightMeter) -> MigrateResult {
		let name = <Pallet<T>>::name();

		if meter.try_consume(<T as pallet::Config>::WeightInfo::migration_noop()).is_err() {
			return MigrateResult::NoMigrationPerformed
		}

		MigrationInProgress::<T>::mutate_exists(|progress| {
			let Some(cursor_before) = progress.as_mut() else {
				meter.consume(<T as pallet::Config>::WeightInfo::migration_noop());
				return MigrateResult::NoMigrationInProgress
			};

			// if a migration is running it is always upgrading to the next version
			let storage_version = <Pallet<T>>::on_chain_storage_version();
			let in_progress_version = storage_version + 1;

			let result =
				match T::Migrations::steps(in_progress_version, cursor_before.as_ref(), &mut meter)
				{
					StepResult::InProgress { cursor, steps_done } => {
						*progress = Some(cursor);
						MigrateResult::InProgress { steps_done }
					},
					StepResult::Completed { steps_done } => {
						in_progress_version.put::<Pallet<T>>();
						if <Pallet<T>>::in_code_storage_version() != in_progress_version {
							*progress = Some(T::Migrations::new(in_progress_version + 1));
							MigrateResult::InProgress { steps_done }
						} else {
							*progress = None;
							MigrateResult::Completed
						}
					},
				};

			result
		})
	}

	pub(crate) fn ensure_migrated() -> DispatchResult {
		if Self::in_progress() {
			Err(Error::<T>::MigrationInProgress.into())
		} else {
			Ok(())
		}
	}

	pub(crate) fn in_progress() -> bool {
		MigrationInProgress::<T>::exists()
	}
}

#[impl_trait_for_tuples::impl_for_tuples(3)]
#[tuple_types_custom_trait_bound(MigrationStep)]
impl MigrateSequence for Tuple {
	const VERSION_RANGE: (u16, u16) = {
		let mut versions: (u16, u16) = (0, 0);
		for_tuples!(
			#(
				match versions {
					(0, 0) => {
						versions = (Tuple::VERSION, Tuple::VERSION);
					},
					(min_version, last_version) if Tuple::VERSION == last_version + 1 => {
						versions = (min_version, Tuple::VERSION);
					},
					_ => panic!("Migrations must be ordered by their versions with no gaps.")
				}
			)*
		);
		versions
	};

	fn new(version: StorageVersion) -> Cursor {
		for_tuples!(
			#(
				if version == Tuple::VERSION {
					return Tuple::default().encode().try_into().expect(PROOF_ENCODE)
				}
			)*
		);
		invalid_version(version)
	}

	#[cfg(feature = "try-runtime")]
	/// Execute the pre-checks of the step associated with this version.
	fn pre_upgrade_step(version: StorageVersion) -> Result<Vec<u8>, TryRuntimeError> {
		for_tuples!(
			#(
				if version == Tuple::VERSION {
					return Tuple::pre_upgrade_step()
				}
			)*
		);
		invalid_version(version)
	}

	#[cfg(feature = "try-runtime")]
	/// Execute the post-checks of the step associated with this version.
	fn post_upgrade_step(version: StorageVersion, state: Vec<u8>) -> Result<(), TryRuntimeError> {
		for_tuples!(
			#(
				if version == Tuple::VERSION {
					return Tuple::post_upgrade_step(state)
				}
			)*
		);
		invalid_version(version)
	}

	fn steps(version: StorageVersion, mut cursor: &[u8], meter: &mut WeightMeter) -> StepResult {
		for_tuples!(
			#(
				if version == Tuple::VERSION {
					let mut migration = <Tuple as Decode>::decode(&mut cursor)
						.expect(PROOF_DECODE);
					let max_weight = Tuple::max_step_weight();
					let mut steps_done = 0;
					while meter.can_consume(max_weight) {
						steps_done.saturating_accrue(1);
						if matches!(migration.step(meter), IsFinished::Yes) {
							return StepResult::Completed{ steps_done }
						}
					}
					return StepResult::InProgress{cursor: migration.encode().try_into().expect(PROOF_ENCODE), steps_done }
				}
			)*
		);
		invalid_version(version)
	}

	fn integrity_test(max_block_weight: Weight) {
		for_tuples!(
			#(
				Tuple::integrity_test(max_block_weight);
			)*
		);
	}
}
