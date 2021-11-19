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
    traits::{
        fungibles::Transfer,
        Currency,
        ExistenceRequirement::{AllowDeath, KeepAlive},
    },
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
        type Assets: Transfer<AccountOf<Self>, AssetId = Self::AssetId, Balance = BalanceOf<Self>>;

        /// The pallet id, used for deriving "pot" accounts of budgets
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// The maximum lifetime of a slot
        #[pallet::constant]
        type SlotLifetime: Get<HeightOf<Self>>;

        /// The swaps trait
        type Swaps: Swaps<
            AccountId = AccountOf<Self>,
            AssetId = Self::AssetId,
            QuoteBalance = BalanceOf<Self>,
            TokenBalance = BalanceOf<Self>,
        >;

        /// The means of storing the tags and tags of advertisement
        type Tags: Tags<DecentralizedId = DidOf<Self>, Hash = HashOf<Self>>;

        /// The origin which may do calls
        type CallOrigin: EnsureOrigin<Self::Origin, Success = (DidOf<Self>, AccountOf<Self>)>;

        /// The origin which may forcibly drawback or destroy an advertisement or otherwise alter privileged attributes
        type ForceOrigin: EnsureOrigin<Self::Origin>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Metadata of an advertisement
    #[pallet::storage]
    #[pallet::getter(fn meta)]
    pub(super) type Metadata<T: Config> = StorageMap<_, Identity, HashOf<T>, MetaOf<T>>;

    /// Advertisement of an advertiser
    #[pallet::storage]
    #[pallet::getter(fn ads_of)]
    pub(super) type AdsOf<T: Config> = StorageMap<_, Identity, DidOf<T>, Vec<HashOf<T>>>;

    /// Deadline of an advertisement in slot
    #[pallet::storage]
    #[pallet::getter(fn deadline_of)]
    pub(super) type DeadlineOf<T: Config> = StorageDoubleMap<
        _,
        Identity,
        DidOf<T>, // use default value for the ad itself
        Identity,
        HashOf<T>,
        HeightOf<T>,
    >;

    /// Slot of a KOL
    #[pallet::storage]
    #[pallet::getter(fn slot_of)]
    pub(super) type SlotOf<T: Config> = StorageMap<_, Identity, DidOf<T>, SlotMetaOf<T>>;

    /// Slots of an advertisement
    #[pallet::storage]
    #[pallet::getter(fn slots_of)]
    pub(super) type SlotsOf<T: Config> = StorageMap<_, Identity, HashOf<T>, Vec<DidOf<T>>>;

    /// Payouts of an advertisement
    #[pallet::storage]
    #[pallet::getter(fn payout)]
    pub(super) type Payout<T: Config> = StorageDoubleMap<
        _,
        Identity,
        HashOf<T>,
        Identity,
        DidOf<T>, //
        HeightOf<T>,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New advertisement created \[id, creator\]
        Created(HashOf<T>, DidOf<T>),
        /// Budget added to an advertisement \[id, owner, value\]
        Deposited(HashOf<T>, DidOf<T>, BalanceOf<T>),
        /// Advertisement updated \[id\]
        Updated(HashOf<T>),
        /// Advertiser bid for slot \[kol, id, value\]
        Bid(DidOf<T>, HashOf<T>, BalanceOf<T>),
        /// Advertisement (in slot) deadline reached \[kol, id, value\]
        End(DidOf<T>, HashOf<T>, BalanceOf<T>),
        /// Advertisement payout \[id, nft, visitor, value, referer, value\]
        Paid(
            HashOf<T>,
            AssetOf<T>,
            DidOf<T>,
            BalanceOf<T>,
            Option<DidOf<T>>,
            BalanceOf<T>,
        ),
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
        Paid,
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
            let mut raw = <AccountOf<T>>::encode(&who);
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
                    created,
                },
            );

            <DeadlineOf<T>>::insert(&Did::<T>::zero(), &id, deadline);

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

            Self::deposit_event(Event::Created(id, creator));
            Self::deposit_event(Event::Deposited(id, creator, budget));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::update_reward_rate())]
        pub fn update_reward_rate(
            origin: OriginFor<T>,
            ad: HashOf<T>,
            reward_rate: u16,
        ) -> DispatchResult {
            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            let deadline = <DeadlineOf<T>>::get(&Did::<T>::zero(), &ad) //
                .ok_or(Error::<T>::NotExists)?;

            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let mut meta = Self::ensure_owned(did, ad)?;

            meta.reward_rate = reward_rate;

            <Metadata<T>>::insert(&ad, meta);

            Self::deposit_event(Event::Updated(ad));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::update_tags(tags.len() as u32))]
        pub fn update_tags(
            origin: OriginFor<T>,
            ad: HashOf<T>,
            tags: Vec<Vec<u8>>,
        ) -> DispatchResult {
            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            let deadline = <DeadlineOf<T>>::get(&Did::<T>::zero(), &ad) //
                .ok_or(Error::<T>::NotExists)?;

            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let _ = Self::ensure_owned(did, ad)?;

            for tag in &tags {
                ensure!(T::Tags::exists(tag), Error::<T>::TagNotExists);
            }

            T::Tags::clr_tag(&ad)?;
            for tag in tags {
                T::Tags::add_tag(&ad, tag)?;
            }

            Self::deposit_event(Event::Updated(ad));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::add_budget())]
        pub fn add_budget(
            origin: OriginFor<T>,
            ad: HashOf<T>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let (did, who) = T::CallOrigin::ensure_origin(origin)?;

            let deadline = <DeadlineOf<T>>::get(&Did::<T>::zero(), &ad) //
                .ok_or(Error::<T>::NotExists)?;

            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let mut meta = Self::ensure_owned(did, ad)?;

            T::Currency::transfer(&who, &meta.pot, value, KeepAlive)?;

            meta.budget.saturating_accrue(value);
            meta.remain.saturating_accrue(value);

            <Metadata<T>>::insert(&ad, meta);

            Self::deposit_event(Event::Deposited(ad, did, value));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::bid())]
        pub fn bid(
            origin: OriginFor<T>,
            ad: HashOf<T>,
            kol: DidOf<T>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            let deadline = <DeadlineOf<T>>::get(&Did::<T>::zero(), &ad) //
                .ok_or(Error::<T>::NotExists)?;

            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let mut meta = Self::ensure_owned(did, ad)?;

            let kol_meta = Did::<T>::meta(&kol).ok_or(Error::<T>::NotMinted)?;
            let nft = kol_meta.nft.ok_or(Error::<T>::NotMinted)?;

            let created = <frame_system::Pallet<T>>::block_number();

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

                let remain = Self::drawback(&kol, &slot)?;

                Self::deposit_event(Event::End(kol, slot.ad, remain));
            }

            // 4. swap AD3 to assets

            let (_, tokens) = T::Swaps::quote_in(&meta.pot, nft, value, One::one(), false)?;

            // 5. update slot

            let lifetime = T::SlotLifetime::get();
            let slotlife = created.saturating_add(lifetime);
            let deadline = if slotlife > deadline {
                deadline
            } else {
                slotlife
            };

            <SlotOf<T>>::insert(
                &kol,
                types::Slot {
                    nft,
                    budget: tokens,
                    remain: tokens,
                    created,
                    ad,
                },
            );

            <DeadlineOf<T>>::insert(&kol, &ad, deadline);

            <SlotsOf<T>>::mutate(&ad, |maybe| {
                if let Some(slots) = maybe {
                    slots.push(kol);
                } else {
                    *maybe = Some(vec![kol]);
                }
            });

            meta.remain.saturating_reduce(value);

            <Metadata<T>>::insert(&ad, meta);

            Self::deposit_event(Event::Bid(kol, ad, value));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::pay(scores.len() as u32))]
        pub fn pay(
            origin: OriginFor<T>,
            ad: HashOf<T>,
            kol: DidOf<T>,
            visitor: DidOf<T>,
            scores: Vec<(Vec<u8>, i8)>,
            referer: Option<DidOf<T>>,
        ) -> DispatchResult {
            ensure!(!scores.is_empty(), Error::<T>::EmptyTags);

            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            let deadline = <DeadlineOf<T>>::get(&Did::<T>::zero(), &ad) //
                .ok_or(Error::<T>::NotExists)?;

            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let meta = Self::ensure_owned(did, ad)?;

            ensure!(!<Payout<T>>::contains_key(&ad, &visitor), Error::<T>::Paid);

            // 1. get slot, check current ad
            let mut slot = <SlotOf<T>>::get(&kol).ok_or(Error::<T>::NotExists)?;
            ensure!(slot.ad == ad, Error::<T>::Underbid);

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
                ensure!(T::Tags::has_tag(&ad, &tag), Error::<T>::TagNotExists);
                ensure!(score >= -5 && score <= 5, Error::<T>::ScoreOutOfRange);

                T::Tags::influence(&visitor, &tag, score as i32)?;
            }

            // 4. payout assets

            let account = Did::<T>::lookup_did(visitor).ok_or(Error::<T>::DidNotExists)?;

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

            T::Assets::transfer(slot.nft, &meta.pot, &account, reward, false)?;

            slot.remain.saturating_reduce(amount);

            <SlotOf<T>>::insert(&kol, &slot);

            <Payout<T>>::insert(&ad, &visitor, height);

            Self::deposit_event(Event::Paid(ad, slot.nft, visitor, reward, referer, award));

            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    fn begin_block(now: HeightOf<T>) -> Result<Weight, DispatchError> {
        use sp_std::collections::btree_set::BTreeSet;

        let weight = 1_000_000_000;

        let mut ads = BTreeSet::new();
        for (kol, ad, deadline) in <DeadlineOf<T>>::iter() {
            if deadline > now {
                continue;
            }

            if kol == Did::<T>::zero() {
                ads.insert(ad);
                continue;
            }

            let slot = <SlotOf<T>>::get(kol);
            if slot.is_none() {
                continue;
            }
            let slot = slot.unwrap();

            if slot.ad != ad {
                continue;
            }

            let _ = Self::drawback(&kol, &slot);
        }

        for ad in ads {
            let meta = <Metadata<T>>::get(ad);
            if meta.is_none() {
                continue;
            }
            let mut meta = meta.unwrap();

            let creator = Did::<T>::meta(&meta.creator);
            if creator.is_none() {
                continue;
            }
            let creator = creator.unwrap();

            let _ = T::Currency::transfer(&meta.pot, &creator.account, meta.remain, AllowDeath);

            meta.remain = Zero::zero();

            <Metadata<T>>::insert(&ad, meta);
        }

        Ok(weight)
    }

    fn drawback(kol: &DidOf<T>, slot: &SlotMetaOf<T>) -> Result<BalanceOf<T>, DispatchError> {
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

        Ok(amount)
    }

    fn ensure_owned(did: DidOf<T>, ad: HashOf<T>) -> Result<MetaOf<T>, DispatchError> {
        let meta = <Metadata<T>>::get(ad).ok_or(Error::<T>::NotExists)?;
        ensure!(meta.creator == did, Error::<T>::NotOwned);

        Ok(meta)
    }
}
