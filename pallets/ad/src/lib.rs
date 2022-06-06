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

mod migrations;
mod types;

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{
        tokens::fungibles::{Inspect as FungInspect, Transfer as FungTransfer},
        Currency,
        ExistenceRequirement::{AllowDeath, KeepAlive},
        StorageVersion,
    },
    weights::Weight,
    Blake2_256, PalletId, StorageHasher,
};
use parami_did::Pallet as Did;
use parami_nft::Pallet as Nft;
use parami_traits::{Swaps, Tags};
use sp_runtime::{
    traits::{AccountIdConversion, Hash, One, Saturating, Zero},
    DispatchError,
};
use sp_std::prelude::*;

use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type AssetsOf<T> = <T as parami_nft::Config>::AssetId;
type BalanceOf<T> = <<T as parami_did::Config>::Currency as Currency<AccountOf<T>>>::Balance;
type DidOf<T> = <T as parami_did::Config>::DecentralizedId;
type HashOf<T> = <<T as frame_system::Config>::Hashing as Hash>::Output;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type MetaOf<T> = types::Metadata<AccountOf<T>, BalanceOf<T>, DidOf<T>, HashOf<T>, HeightOf<T>>;
type NftOf<T> = <T as parami_nft::Config>::AssetId;
type SlotMetaOf<T> = types::Slot<BalanceOf<T>, HashOf<T>, HeightOf<T>, NftOf<T>, AssetsOf<T>>;
type TagOf = <Blake2_256 as StorageHasher>::Output;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config:
        frame_system::Config //
        + parami_did::Config
        + parami_nft::Config
    {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The minimum fee balance required to keep alive an ad
        #[pallet::constant]
        type MinimumFeeBalance: Get<BalanceOf<Self>>;

        /// The pallet id, used for deriving "pot" accounts of budgets
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// The maximum lifetime of a slot
        #[pallet::constant]
        type SlotLifetime: Get<HeightOf<Self>>;

        /// The means of storing the tags and tags of advertisement
        type Tags: Tags<TagOf, HashOf<Self>, DidOf<Self>>;

        /// The origin which may do calls
        type CallOrigin: EnsureOrigin<Self::Origin, Success = (DidOf<Self>, AccountOf<Self>)>;

        /// The origin which may forcibly drawback or destroy an advertisement or otherwise alter privileged attributes
        type ForceOrigin: EnsureOrigin<Self::Origin>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
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

    /// End time of an advertisement
    #[pallet::storage]
    #[pallet::getter(fn endtime_of)]
    pub(super) type EndtimeOf<T: Config> = StorageMap<_, Identity, HashOf<T>, HeightOf<T>>;

    /// Deadline of an advertisement in a slot
    #[pallet::storage]
    #[pallet::getter(fn deadline_of)]
    pub(super) type DeadlineOf<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        NftOf<T>, // KOL NFT ID
        Identity,
        HashOf<T>,
        HeightOf<T>,
    >;

    /// Slot of a NFT
    #[pallet::storage]
    #[pallet::getter(fn slot_of)]
    pub(super) type SlotOf<T: Config> = StorageMap<_, Twox64Concat, NftOf<T>, SlotMetaOf<T>>;

    /// Slots of an advertisement
    #[pallet::storage]
    #[pallet::getter(fn slots_of)]
    pub(super) type SlotsOf<T: Config> = StorageMap<_, Identity, HashOf<T>, Vec<NftOf<T>>>;

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
        Bid(NftOf<T>, HashOf<T>, BalanceOf<T>),
        /// Advertisement (in slot) deadline reached \[kol, id, value\]
        End(NftOf<T>, HashOf<T>, BalanceOf<T>),
        /// Advertisement payout \[id, nft, visitor, value, referrer, value\]
        Paid(
            HashOf<T>,
            NftOf<T>,
            DidOf<T>,
            BalanceOf<T>,
            Option<DidOf<T>>,
            BalanceOf<T>,
        ),
        /// Swap Triggered \[id, kol, remain\]
        SwapTriggered(HashOf<T>, NftOf<T>, BalanceOf<T>),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            Self::begin_block(n).unwrap_or_else(|e| {
                sp_runtime::print(e);
                0
            })
        }

        fn on_runtime_upgrade() -> Weight {
            migrations::migrate::<T>()
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        Deadline,
        DidNotExists,
        EmptyTags,
        InsufficientBalance,
        InsufficientFractions,
        InsufficientFungibles,
        NotExists,
        NotMinted,
        NotOwned,
        Paid,
        ScoreOutOfRange,
        TagNotExists,
        Underbid,
        FungiblesNotEqualToFractions,
        WrongPayoutSetting,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(<T as Config>::WeightInfo::create(
            metadata.len() as u32,
            tags.len() as u32
        ))]
        pub fn create(
            origin: OriginFor<T>,
            #[pallet::compact] budget: BalanceOf<T>,
            tags: Vec<Vec<u8>>,
            metadata: Vec<u8>,
            reward_rate: u16,
            deadline: HeightOf<T>,
            payout_base: BalanceOf<T>,
            payout_min: BalanceOf<T>,
            payout_max: BalanceOf<T>,
        ) -> DispatchResult {
            let created = <frame_system::Pallet<T>>::block_number();

            ensure!(deadline > created, Error::<T>::Deadline);
            //TODO: ensure!(payout_base > xxx)
            ensure!(payout_min < payout_max, Error::<T>::WrongPayoutSetting);
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
                    payout_base,
                    payout_min,
                    payout_max,
                },
            );

            <EndtimeOf<T>>::insert(&id, deadline);

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
            id: HashOf<T>,
            reward_rate: u16,
        ) -> DispatchResult {
            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            let height = <frame_system::Pallet<T>>::block_number();

            let endtime = <EndtimeOf<T>>::get(&id).ok_or(Error::<T>::NotExists)?;
            ensure!(endtime > height, Error::<T>::Deadline);

            let mut meta = Self::ensure_owned(did, id)?;

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

            let height = <frame_system::Pallet<T>>::block_number();

            let endtime = <EndtimeOf<T>>::get(&id).ok_or(Error::<T>::NotExists)?;
            ensure!(endtime > height, Error::<T>::Deadline);

            let _ = Self::ensure_owned(did, id)?;

            for tag in &tags {
                ensure!(T::Tags::exists(tag), Error::<T>::TagNotExists);
            }

            T::Tags::clr_tag(&id)?;
            for tag in tags {
                T::Tags::add_tag(&id, tag)?;
            }

            Self::deposit_event(Event::Updated(id));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::add_budget())]
        pub fn add_budget(
            origin: OriginFor<T>,
            id: HashOf<T>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let (did, who) = T::CallOrigin::ensure_origin(origin)?;

            let height = <frame_system::Pallet<T>>::block_number();

            let endtime = <EndtimeOf<T>>::get(&id).ok_or(Error::<T>::NotExists)?;
            ensure!(endtime > height, Error::<T>::Deadline);

            let mut meta = Self::ensure_owned(did, id)?;

            T::Currency::transfer(&who, &meta.pot, value, KeepAlive)?;

            meta.budget.saturating_accrue(value);
            meta.remain.saturating_accrue(value);

            <Metadata<T>>::insert(&id, meta);

            Self::deposit_event(Event::Deposited(id, did, value));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::bid())]
        pub fn bid(
            origin: OriginFor<T>,
            ad_id: HashOf<T>,
            nft_id: NftOf<T>,
            #[pallet::compact] value: BalanceOf<T>, // AD3
            fungible_id: Option<AssetsOf<T>>,
            fungibles: Option<BalanceOf<T>>,
        ) -> DispatchResult {
            let (did, who) = T::CallOrigin::ensure_origin(origin)?;

            let height = <frame_system::Pallet<T>>::block_number();

            let endtime = <EndtimeOf<T>>::get(&ad_id).ok_or(Error::<T>::NotExists)?;
            ensure!(endtime > height, Error::<T>::Deadline);

            let fungibles = match fungibles {
                Some(fungibles) if fungibles > Zero::zero() && fungible_id.is_some() => {
                    let fungible_id = fungible_id.unwrap();
                    ensure!(
                        fungibles <= T::Assets::balance(fungible_id, &who),
                        Error::<T>::InsufficientFungibles
                    );
                    fungibles
                }
                _ => Zero::zero(),
            };

            let mut ad_meta = Self::ensure_owned(did, ad_id)?;
            ensure!(ad_meta.remain >= value, Error::<T>::InsufficientBalance);

            let nft_meta = Nft::<T>::meta(nft_id).ok_or(Error::<T>::NotMinted)?;
            ensure!(nft_meta.minted, Error::<T>::NotMinted);

            let created = <frame_system::Pallet<T>>::block_number();

            // 1. check slot of kol
            let slot = <SlotOf<T>>::get(nft_id);

            // 2. if slot is used
            // require a 20% increase of current budget
            // and drawback current ad

            if let Some(slot) = slot {
                let quote = T::Swaps::token_in_dry(slot.nft_id, slot.fractions_remain)?;
                let remain = slot.remain.saturating_add(quote);

                ensure!(
                    value.saturating_mul(100u32.into()) / 120u32.into() > remain,
                    Error::<T>::Underbid
                );

                let _ = Self::drawback(&slot)?;
            }

            // 3. deposit fungibles

            if let Some(fungible_id) = fungible_id {
                let _ = T::Assets::transfer(fungible_id, &who, &ad_meta.pot, fungibles, false)?;
            }

            // 4. update slot

            let lifetime = T::SlotLifetime::get();
            let slotlife = created.saturating_add(lifetime);
            let deadline = if slotlife > endtime {
                endtime
            } else {
                slotlife
            };

            let mut slot = types::Slot {
                ad_id,
                nft_id,
                fungible_id,
                budget: value,
                remain: value,
                fractions_remain: Zero::zero(),
                fungibles_budget: fungibles,
                fungibles_remain: fungibles,
                created,
            };

            Self::swap_by_10percent(&ad_meta, nft_meta.token_asset_id, &mut slot, One::one())?;

            <SlotOf<T>>::insert(nft_id, &slot);

            <DeadlineOf<T>>::insert(nft_id, &ad_id, deadline);

            ad_meta.remain.saturating_reduce(value);

            <Metadata<T>>::insert(&ad_id, &ad_meta);

            <SlotsOf<T>>::mutate(&ad_id, |maybe| {
                if let Some(slots) = maybe {
                    slots.push(nft_id);
                } else {
                    *maybe = Some(vec![nft_id]);
                }
            });

            Self::deposit_event(Event::Bid(nft_id, ad_id, value));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::pay(scores.len() as u32))]
        pub fn pay(
            origin: OriginFor<T>,
            ad_id: HashOf<T>,
            nft_id: NftOf<T>,
            visitor: DidOf<T>,
            scores: Vec<(Vec<u8>, i8)>,
            referrer: Option<DidOf<T>>,
        ) -> DispatchResult {
            ensure!(!scores.is_empty(), Error::<T>::EmptyTags);

            let (did, who) = T::CallOrigin::ensure_origin(origin)?;

            let height = <frame_system::Pallet<T>>::block_number();

            let endtime = <EndtimeOf<T>>::get(&ad_id).ok_or(Error::<T>::NotExists)?;
            ensure!(endtime > height, Error::<T>::Deadline);

            let ad_meta = Self::ensure_owned(did, ad_id)?;

            let nft_meta = Nft::<T>::meta(nft_id).ok_or(Error::<T>::NotMinted)?;
            ensure!(nft_meta.minted, Error::<T>::NotMinted);

            let deadline = <DeadlineOf<T>>::get(nft_id, &ad_id).ok_or(Error::<T>::NotExists)?;
            ensure!(deadline > height, Error::<T>::Deadline);

            ensure!(
                !<Payout<T>>::contains_key(&ad_id, &visitor),
                Error::<T>::Paid
            );

            // 1. get slot, check current ad
            let mut slot = <SlotOf<T>>::get(nft_id).ok_or(Error::<T>::NotExists)?;
            ensure!(slot.ad_id == ad_id, Error::<T>::Underbid);

            // 2. scoring visitor

            let mut scoring = 5i32;

            let tags = T::Tags::tags_of(&ad_id);
            let personas = T::Tags::personas_of(&visitor);
            let length = tags.len();
            for (tag, score) in personas {
                let delta = if tags.contains_key(&tag) {
                    score.saturating_mul(10)
                } else {
                    score
                };
                scoring.saturating_accrue(delta);
            }

            scoring /= length.saturating_mul(10).saturating_add(1) as i32;

            if scoring < 0 {
                scoring = 0;
            }

            let scoring = scoring as u32;

            let mut amount = ad_meta.payout_base.saturating_mul(scoring.into());
            if amount < ad_meta.payout_min {
                amount = ad_meta.payout_min;
            }
            if amount > ad_meta.payout_max {
                amount = ad_meta.payout_max;
            }
            if slot.fractions_remain < amount {
                // if tokens is not enough, swap tokens

                // swap 10% of current budget, at least cover current payout
                Self::swap_by_10percent(&ad_meta, nft_meta.token_asset_id, &mut slot, amount)?;

                <SlotOf<T>>::insert(nft_id, &slot);
            }

            ensure!(
                slot.fractions_remain >= amount,
                Error::<T>::InsufficientFractions
            );
            let fungibles = if slot.fungibles_budget > Zero::zero() {
                let fungibles = amount.clone();
                ensure!(
                    slot.fungibles_remain >= fungibles,
                    Error::<T>::InsufficientFungibles
                );
                fungibles
            } else {
                Zero::zero()
            };
            // 3. influence visitor

            for (tag, score) in scores {
                ensure!(T::Tags::has_tag(&ad_id, &tag), Error::<T>::TagNotExists);
                ensure!(score >= -5 && score <= 5, Error::<T>::ScoreOutOfRange);

                T::Tags::influence(&visitor, &tag, score as i32)?;
            }

            // 4. payout assets

            let account = Did::<T>::lookup_did(visitor).ok_or(Error::<T>::DidNotExists)?;

            let award = if let Some(referrer) = referrer {
                let rate = ad_meta.reward_rate.into();
                let award = amount.saturating_mul(rate) / 100u32.into();

                let referrer = Did::<T>::lookup_did(referrer).ok_or(Error::<T>::DidNotExists)?;

                T::Assets::transfer(slot.nft_id, &ad_meta.pot, &referrer, award, false)?;

                award
            } else {
                Zero::zero()
            };

            let reward = amount.saturating_sub(award);

            T::Assets::transfer(slot.nft_id, &ad_meta.pot, &account, reward, false)?;

            slot.fractions_remain.saturating_reduce(amount);

            if let Some(fungible_id) = slot.fungible_id {
                T::Assets::transfer(fungible_id, &ad_meta.pot, &account, fungibles, false)?;
                slot.fungibles_remain.saturating_reduce(fungibles);
            }

            <SlotOf<T>>::insert(nft_id, &slot);

            <Payout<T>>::insert(&ad_id, &visitor, height);

            Self::deposit_event(Event::Paid(
                ad_id,
                slot.nft_id,
                visitor,
                reward,
                referrer,
                award,
            ));

            // 5. drawback if advertiser does not have enough fees

            if T::Currency::free_balance(&who) < T::MinimumFeeBalance::get() {
                let _ = Self::drawback(&slot);
            }

            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig {}

    #[cfg(feature = "std")]
    impl Default for GenesisConfig {
        fn default() -> Self {
            Self {}
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {}
    }
}

impl<T: Config> Pallet<T> {
    fn begin_block(now: HeightOf<T>) -> Result<Weight, DispatchError> {
        use frame_support::traits::Get;

        let mut read = 0;
        let mut write = 0;

        let mut amount = 0;

        for (nft_id, ad_id, deadline) in <DeadlineOf<T>>::iter() {
            read += 1;

            if amount >= 100 {
                break;
            }

            if deadline > now {
                continue;
            }

            read += 1;
            let slot = <SlotOf<T>>::get(nft_id);
            if let Some(slot) = slot {
                if slot.ad_id != ad_id {
                    continue;
                }

                read += 2;
                write += 4;
                let _ = Self::drawback(&slot);

                amount += 1;
            }
        }

        let mut amount = 0;

        for (ad_id, endtime) in <EndtimeOf<T>>::iter() {
            read += 1;

            if amount >= 100 {
                break;
            }

            if endtime > now {
                continue;
            }

            read += 1;
            if let Some(slots) = <SlotsOf<T>>::get(ad_id) {
                if slots.len() > 0 {
                    continue;
                }
            }

            read += 1;
            let meta = <Metadata<T>>::get(ad_id);
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

            write += 2;
            <Metadata<T>>::insert(ad_id, meta);
            <EndtimeOf<T>>::remove(ad_id);

            amount += 1;
        }

        Ok(T::DbWeight::get().reads_writes(read as Weight, write as Weight))
    }

    fn drawback(slot: &SlotMetaOf<T>) -> Result<BalanceOf<T>, DispatchError> {
        let mut meta = <Metadata<T>>::get(slot.ad_id).ok_or(Error::<T>::NotExists)?;

        if let Some(fungible_id) = slot.fungible_id {
            if let Some(who) = Did::<T>::lookup_did(meta.creator) {
                T::Assets::transfer(fungible_id, &meta.pot, &who, slot.fungibles_remain, false)?;
            }
        }

        let amount = T::Swaps::token_in(
            meta.pot.clone(),
            slot.nft_id,
            slot.fractions_remain,
            One::one(),
            false,
        )?;

        meta.remain.saturating_accrue(slot.remain);
        meta.remain.saturating_accrue(amount);

        <Metadata<T>>::insert(slot.ad_id, meta);

        <SlotOf<T>>::remove(slot.nft_id);

        <SlotsOf<T>>::mutate(slot.ad_id, |maybe| {
            if let Some(slots) = maybe {
                slots.retain(|x| *x != slot.nft_id);
            }
        });

        <DeadlineOf<T>>::remove(slot.nft_id, slot.ad_id);

        Self::deposit_event(Event::End(slot.nft_id, slot.ad_id, amount));

        Ok(amount)
    }

    fn ensure_owned(did: DidOf<T>, id: HashOf<T>) -> Result<MetaOf<T>, DispatchError> {
        let meta = <Metadata<T>>::get(&id).ok_or(Error::<T>::NotExists)?;
        ensure!(meta.creator == did, Error::<T>::NotOwned);

        Ok(meta)
    }

    fn swap_by_10percent(
        meta: &MetaOf<T>,
        token: NftOf<T>,
        slot: &mut SlotMetaOf<T>,
        least: BalanceOf<T>,
    ) -> DispatchResult {
        // swap per 10%
        let amount = slot.budget / 10u32.into();
        let fractions = T::Swaps::quote_in(meta.pot.clone(), token, amount, least, false)?;

        slot.remain.saturating_reduce(amount);
        slot.fractions_remain.saturating_accrue(fractions);

        Self::deposit_event(Event::SwapTriggered(slot.ad_id, token, slot.remain));

        Ok(())
    }
}
