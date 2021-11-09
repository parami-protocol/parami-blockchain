#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[rustfmt::skip]
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod types;

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{fungibles::Transfer, Currency, ExistenceRequirement::KeepAlive},
    weights::Weight,
    PalletId,
};
use parami_did::Pallet as Did;
use parami_traits::{Swaps, Tags};
use sp_runtime::{
    traits::{AccountIdConversion, Hash, One, Saturating, Zero},
    DispatchError,
};
use sp_std::prelude::*;

use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type AssetOf<T> = <T as parami_did::Config>::AssetId;
type BalanceOf<T> = <<T as parami_did::Config>::Currency as Currency<AccountOf<T>>>::Balance;
type DidOf<T> = <T as parami_did::Config>::DecentralizedId;
type HashOf<T> = <<T as frame_system::Config>::Hashing as Hash>::Output;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type MetaOf<T> = types::Metadata<AccountOf<T>, BalanceOf<T>, DidOf<T>, HashOf<T>, HeightOf<T>>;
type SlotMetaOf<T> = types::Slot<BalanceOf<T>, HashOf<T>, HeightOf<T>, AssetOf<T>>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + parami_did::Config {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The assets trait to pay rewards
        type Assets: Transfer<Self::AccountId, AssetId = Self::AssetId, Balance = BalanceOf<Self>>;

        /// The pallet id, used for deriving "pot" accounts of budgets
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// The swaps trait
        type Swaps: Swaps<
            AccountId = Self::AccountId,
            AssetId = Self::AssetId,
            QuoteBalance = BalanceOf<Self>,
            TokenBalance = BalanceOf<Self>,
        >;

        /// The means of storing the tags and tags of advertisement
        type Tags: Tags<DecentralizedId = Self::DecentralizedId, Hash = HashOf<Self>>;

        /// The origin which may do calls
        type CallOrigin: EnsureOrigin<
            Self::Origin,
            Success = (Self::DecentralizedId, Self::AccountId),
        >;

        /// The origin which may forcibly drawback or destroy an advertisement or otherwise alter privileged attributes
        type ForceOrigin: EnsureOrigin<Self::Origin>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    /// Metadata of an advertisement
    #[pallet::storage]
    #[pallet::getter(fn meta)]
    pub(super) type Metadata<T: Config> = StorageMap<_, Identity, HashOf<T>, MetaOf<T>>;

    /// Advertisement of an advertiser
    #[pallet::storage]
    #[pallet::getter(fn ads_of)]
    pub(super) type AdsOf<T: Config> = StorageMap<_, Identity, T::DecentralizedId, Vec<HashOf<T>>>;

    /// Deadline of an advertisement in slot
    #[pallet::storage]
    #[pallet::getter(fn deadline_of)]
    pub(super) type DeadlineOf<T: Config> =
        StorageDoubleMap<_, Identity, T::DecentralizedId, Identity, HashOf<T>, HeightOf<T>>;

    /// Slot of a KOL
    #[pallet::storage]
    #[pallet::getter(fn slot_of)]
    pub(super) type SlotOf<T: Config> = StorageMap<_, Identity, T::DecentralizedId, SlotMetaOf<T>>;

    /// Slots of an advertisement
    #[pallet::storage]
    #[pallet::getter(fn slots_of)]
    pub(super) type SlotsOf<T: Config> =
        StorageMap<_, Identity, HashOf<T>, Vec<T::DecentralizedId>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New advertisement created \[id\]
        Created(HashOf<T>),
        /// Advertisement updated \[id\]
        Updated(HashOf<T>),
        /// Advertiser bid for slot \[kol, id\]
        Bid(T::DecentralizedId, HashOf<T>),
        /// Advertisement (in slot) deadline reached
        End(T::DecentralizedId, HashOf<T>),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: HeightOf<T>) -> Weight {
            Self::begin_block(n).unwrap_or_else(|e| {
                sp_runtime::print(e);
                0
            })
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        Deadline,
        DidNotExists,
        EmptyTags,
        InsufficientTokens,
        NotExists,
        NotMinted,
        NotOwned,
        ScoreOutOfRange,
        TagNotExists,
        Underbid,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(<T as Config>::WeightInfo::create(tags.len() as u32))]
        pub fn create(
            origin: OriginFor<T>,
            #[pallet::compact] budget: BalanceOf<T>,
            tags: Vec<Vec<u8>>,
            metadata: Vec<u8>,
            reward_rate: u16,
            deadline: HeightOf<T>,
        ) -> DispatchResult {
            let created = <frame_system::Pallet<T>>::block_number();

            ensure!(deadline > created, Error::<T>::Deadline);

            let (creator, who) = T::CallOrigin::ensure_origin(origin)?;

            for tag in &tags {
                ensure!(T::Tags::exists(tag), Error::<T>::TagNotExists);
            }

            // 1. derive deposit poll account and advertisement ID

            // TODO: use a HMAC-based algorithm.
            let mut raw = T::AccountId::encode(&who);
            let mut ord = T::BlockNumber::encode(&created);
            raw.append(&mut ord);

            let id = <T as frame_system::Config>::Hashing::hash(&raw);

            let pot = <T as Config>::PalletId::get().into_sub_account(&id);

            // 2. deposit budget

            T::Currency::transfer(&who, &pot, budget, KeepAlive)?;

            // 3. insert metadata, ads_of, tags_of

            <Metadata<T>>::insert(
                &id,
                types::Metadata {
                    id,
                    creator,
                    pot,
                    budget,
                    remain: budget,
                    metadata,
                    reward_rate,
                    deadline,
                    created,
                },
            );

            <AdsOf<T>>::mutate(&creator, |maybe| {
                if let Some(ads) = maybe {
                    ads.push(id);
                } else {
                    *maybe = Some(vec![id]);
                }
            });

            for tag in tags {
                T::Tags::add_tag(&id, tag)?;
            }

            Self::deposit_event(Event::Created(id));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::update_reward_rate())]
        pub fn update_reward_rate(
            origin: OriginFor<T>,
            id: HashOf<T>,
            reward_rate: u16,
        ) -> DispatchResult {
            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            let mut meta = Self::ensure_owned(did, id)?;

            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(meta.deadline > height, Error::<T>::Deadline);

            meta.reward_rate = reward_rate;

            <Metadata<T>>::insert(&id, meta);

            Self::deposit_event(Event::Updated(id));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::update_tags(tags.len() as u32))]
        pub fn update_tags(
            origin: OriginFor<T>,
            id: HashOf<T>,
            tags: Vec<Vec<u8>>,
        ) -> DispatchResult {
            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            let meta = Self::ensure_owned(did, id)?;

            for tag in &tags {
                ensure!(T::Tags::exists(tag), Error::<T>::TagNotExists);
            }

            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(meta.deadline > height, Error::<T>::Deadline);

            T::Tags::clr_tag(&id)?;
            for tag in tags {
                T::Tags::add_tag(&id, tag)?;
            }

            Self::deposit_event(Event::Updated(id));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::bid())]
        pub fn bid(
            origin: OriginFor<T>,
            ad: HashOf<T>,
            kol: T::DecentralizedId,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            let mut meta = Self::ensure_owned(did, ad)?;

            let kol_meta = Did::<T>::meta(&kol).ok_or(Error::<T>::NotMinted)?;
            let nft = kol_meta.nft.ok_or(Error::<T>::NotMinted)?;

            let height = <frame_system::Pallet<T>>::block_number();

            ensure!(meta.deadline > height, Error::<T>::Deadline);

            // 1. check slot of kol

            let slot = <SlotOf<T>>::get(&kol);

            // 2. swap AD3 to assets

            let tokens = T::Swaps::quote_in_dry(nft, value)?;

            // 3. if slot is used
            // require a 20% increase of current budget
            // and drawback current ad

            if let Some(slot) = slot {
                ensure!(
                    tokens >= slot.remain.saturating_mul(120u32.into()) / 100u32.into(),
                    Error::<T>::Underbid
                );

                Self::drawback(&kol, &slot)?;
            }

            // 4. swap AD3 to assets

            let (_, tokens) = T::Swaps::quote_in(&meta.pot, nft, value, One::one(), false)?;

            // 5. update slot

            let deadline = height.saturating_add(43200u32.into()); // 3 Days (3 * 24 * 60 * 60 /6)
            let deadline = if deadline > meta.deadline {
                meta.deadline
            } else {
                deadline
            };

            <DeadlineOf<T>>::insert(&kol, &ad, deadline);

            <SlotOf<T>>::insert(
                &kol,
                types::Slot {
                    nft,
                    budget: tokens,
                    remain: tokens,
                    deadline,
                    ad,
                },
            );

            <SlotsOf<T>>::mutate(&ad, |maybe| {
                if let Some(slots) = maybe {
                    slots.push(kol);
                } else {
                    *maybe = Some(vec![kol]);
                }
            });

            meta.remain.saturating_reduce(value);

            <Metadata<T>>::insert(&ad, meta);

            Self::deposit_event(Event::Bid(kol, ad));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::deposit())]
        pub fn deposit(
            origin: OriginFor<T>,
            id: HashOf<T>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let (did, who) = T::CallOrigin::ensure_origin(origin)?;

            let mut meta = Self::ensure_owned(did, id)?;

            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(meta.deadline > height, Error::<T>::Deadline);

            T::Currency::transfer(&who, &meta.pot, value, KeepAlive)?;

            meta.budget.saturating_accrue(value);
            meta.remain.saturating_accrue(value);

            <Metadata<T>>::insert(&id, meta);

            Self::deposit_event(Event::Updated(id));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::pay(scores.len() as u32))]
        pub fn pay(
            origin: OriginFor<T>,
            id: HashOf<T>,
            kol: T::DecentralizedId,
            visitor: T::DecentralizedId,
            scores: Vec<(Vec<u8>, i8)>,
            referer: Option<T::DecentralizedId>,
        ) -> DispatchResult {
            ensure!(!scores.is_empty(), Error::<T>::EmptyTags);

            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            let meta = Self::ensure_owned(did, id)?;

            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(meta.deadline > height, Error::<T>::Deadline);

            // 1. get slot, check current ad
            let mut slot = <SlotOf<T>>::get(&kol).ok_or(Error::<T>::NotExists)?;
            ensure!(slot.ad == id, Error::<T>::Underbid);

            // 2. scoring visitor

            let mut socring = 5i32;

            let personas = T::Tags::personas_of(&visitor);
            let length = personas.len();
            for (_, score) in personas {
                socring += score;
            }

            socring /= (length + 1) as i32;

            if socring < 0 {
                socring = 0;
            }

            let socring = socring as u32;

            // TODO: find a perfect balance

            let amount = T::Currency::minimum_balance().saturating_mul(socring.into());

            ensure!(slot.remain >= amount, Error::<T>::InsufficientTokens);

            // 3. influence visitor

            for (tag, score) in scores {
                ensure!(T::Tags::has_tag(&id, &tag), Error::<T>::TagNotExists);
                ensure!(score >= -5 && score <= 5, Error::<T>::ScoreOutOfRange);

                T::Tags::influence(&visitor, &tag, score as i32)?;
            }

            // 4. payout assets

            let visitor = Did::<T>::lookup_did(visitor).ok_or(Error::<T>::DidNotExists)?;

            let award = if let Some(referer) = referer {
                let rate = meta.reward_rate.into();
                let award = amount.saturating_mul(rate) / 100u32.into();

                let referer = Did::<T>::lookup_did(referer).ok_or(Error::<T>::DidNotExists)?;

                T::Assets::transfer(slot.nft, &meta.pot, &referer, award, false)?;

                award
            } else {
                Zero::zero()
            };

            let reward = amount.saturating_sub(award);

            T::Assets::transfer(slot.nft, &meta.pot, &visitor, reward, false)?;

            slot.remain.saturating_reduce(amount);

            <SlotOf<T>>::insert(&kol, slot);

            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    fn begin_block(now: HeightOf<T>) -> Result<Weight, DispatchError> {
        let weight = 1_000_000_000;

        // TODO: weight benchmark

        // 1. check deadline of slots
        for (kol, ad, deadline) in <DeadlineOf<T>>::iter() {
            if deadline > now {
                continue;
            }

            let slot = <SlotOf<T>>::get(kol).ok_or(Error::<T>::NotExists)?;

            if slot.ad != ad {
                continue;
            }

            Self::drawback(&kol, &slot)?;

            Self::deposit_event(Event::End(kol, slot.ad));
        }

        // TODO: check deadline of ads

        Ok(weight)
    }

    fn drawback(kol: &T::DecentralizedId, slot: &SlotMetaOf<T>) -> DispatchResult {
        let mut meta = <Metadata<T>>::get(slot.ad).ok_or(Error::<T>::NotExists)?;

        let (_, amount) = T::Swaps::token_in(&meta.pot, slot.nft, slot.remain, One::one(), false)?;

        meta.remain.saturating_accrue(amount);

        <Metadata<T>>::insert(slot.ad, meta);

        <SlotOf<T>>::remove(kol);

        <SlotsOf<T>>::mutate(slot.ad, |maybe| {
            if let Some(slots) = maybe {
                slots.retain(|x| *x != *kol);
            }
        });

        <DeadlineOf<T>>::remove(kol, slot.ad);

        Ok(())
    }

    fn ensure_owned(did: T::DecentralizedId, id: HashOf<T>) -> Result<MetaOf<T>, DispatchError> {
        let meta = <Metadata<T>>::get(id).ok_or(Error::<T>::NotExists)?;
        ensure!(meta.creator == did, Error::<T>::NotOwned);

        Ok(meta)
    }
}
