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
use parami_magic::Pallet as Magic;
use parami_nft::{NftMetaFor, Pallet as Nft};
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
type NftIdOf<T> = parami_nft::NftIdOf<T>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config:
        frame_system::Config //
        + parami_did::Config
        + parami_magic::Config
        + parami_nft::Config
    {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The assets trait to pay rewards
        type Assets: Transfer<AccountOf<Self>, AssetId = Self::AssetId, Balance = BalanceOf<Self>>;

        /// The minimum fee balance required to keep alive an ad
        #[pallet::constant]
        type MinimumFeeBalance: Get<BalanceOf<Self>>;

        /// The pallet id, used for deriving "pot" accounts of budgets
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// The base of payout
        #[pallet::constant]
        type PayoutBase: Get<BalanceOf<Self>>;

        /// The maximum lifetime of a slot
        #[pallet::constant]
        type SlotLifetime: Get<HeightOf<Self>>;

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
        NftIdOf<T>, // KOL NFT ID
        Identity,
        HashOf<T>,
        HeightOf<T>,
    >;

    /// Slot of a NFT
    #[pallet::storage]
    #[pallet::getter(fn slot_of)]
    pub(super) type SlotOf<T: Config> = StorageMap<_, Twox64Concat, NftIdOf<T>, SlotMetaOf<T>>;

    /// Slots of an advertisement
    #[pallet::storage]
    #[pallet::getter(fn slots_of)]
    pub(super) type SlotsOf<T: Config> = StorageMap<_, Identity, HashOf<T>, Vec<NftIdOf<T>>>;

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
        End(NftIdOf<T>, HashOf<T>, BalanceOf<T>),
        /// Advertisement payout \[id, nft, visitor, value, referer, value\]
        Paid(
            HashOf<T>,
            AssetOf<T>,
            DidOf<T>,
            BalanceOf<T>,
            Option<DidOf<T>>,
            BalanceOf<T>,
        ),
        /// Swap Triggered \[id, kol, remain\]
        SwapTriggered(HashOf<T>, DidOf<T>, BalanceOf<T>),
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
        InsufficientBalance,
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

            <T as parami_did::Config>::Currency::transfer(&who, &pot, budget, KeepAlive)?;

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
            ad: HashOf<T>,
            reward_rate: u16,
        ) -> DispatchResult {
            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            let height = <frame_system::Pallet<T>>::block_number();

            let endtime = <EndtimeOf<T>>::get(&ad).ok_or(Error::<T>::NotExists)?;
            ensure!(endtime > height, Error::<T>::Deadline);

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

            let height = <frame_system::Pallet<T>>::block_number();

            let endtime = <EndtimeOf<T>>::get(&ad).ok_or(Error::<T>::NotExists)?;
            ensure!(endtime > height, Error::<T>::Deadline);

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

            let height = <frame_system::Pallet<T>>::block_number();

            let endtime = <EndtimeOf<T>>::get(&ad).ok_or(Error::<T>::NotExists)?;
            ensure!(endtime > height, Error::<T>::Deadline);

            let mut meta = Self::ensure_owned(did, ad)?;

            <T as parami_did::Config>::Currency::transfer(&who, &meta.pot, value, KeepAlive)?;

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

            let height = <frame_system::Pallet<T>>::block_number();

            let endtime = <EndtimeOf<T>>::get(&ad).ok_or(Error::<T>::NotExists)?;
            ensure!(endtime > height, Error::<T>::Deadline);

            let mut meta = Self::ensure_owned(did, ad)?;

            ensure!(meta.remain >= value, Error::<T>::InsufficientBalance);

            let preferred_nft_id = Nft::<T>::get_preferred(kol).ok_or(Error::<T>::NotMinted)?;
            let nft_meta = Nft::<T>::get_meta_of(preferred_nft_id).ok_or(Error::<T>::NotMinted)?;

            ensure!(nft_meta.minted, Error::<T>::NotMinted);

            let created = <frame_system::Pallet<T>>::block_number();

            // 1. check slot of kol
            let slot = <SlotOf<T>>::get(&preferred_nft_id);

            // 2. if slot is used
            // require a 20% increase of current budget
            // and drawback current ad

            if let Some(slot) = slot {
                let quote = T::Swaps::token_in_dry(slot.nft, slot.tokens)?;
                let remain = slot.remain.saturating_add(quote);

                ensure!(
                    value.saturating_mul(100u32.into()) / 120u32.into() > remain,
                    Error::<T>::Underbid
                );

                let _ = Self::drawback(preferred_nft_id, &slot)?;
            }

            // 3. update slot

            let lifetime = T::SlotLifetime::get();
            let slotlife = created.saturating_add(lifetime);
            let deadline = if slotlife > endtime {
                endtime
            } else {
                slotlife
            };

            let mut slot = types::Slot {
                nft: preferred_nft_id,
                budget: value,
                remain: value,
                tokens: Zero::zero(),
                created,
                ad,
            };

            Self::swap_by_10percent(kol, &meta, &nft_meta, &mut slot, One::one())?;

            <SlotOf<T>>::insert(preferred_nft_id, &slot);

            <DeadlineOf<T>>::insert(preferred_nft_id, &ad, deadline);

            meta.remain.saturating_reduce(value);

            <Metadata<T>>::insert(&ad, &meta);

            <SlotsOf<T>>::mutate(&ad, |maybe| {
                if let Some(slots) = maybe {
                    slots.push(preferred_nft_id);
                } else {
                    *maybe = Some(vec![preferred_nft_id]);
                }
            });

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

            let (did, who) = T::CallOrigin::ensure_origin(origin)?;

            let height = <frame_system::Pallet<T>>::block_number();

            let endtime = <EndtimeOf<T>>::get(&ad).ok_or(Error::<T>::NotExists)?;
            ensure!(endtime > height, Error::<T>::Deadline);

            let meta = Self::ensure_owned(did, ad)?;

            let preferred_nft_id = Nft::<T>::get_preferred(kol).ok_or(Error::<T>::NotMinted)?;

            let nft_meta = Nft::<T>::get_meta_of(preferred_nft_id).ok_or(Error::<T>::NotMinted)?;
            ensure!(nft_meta.minted, Error::<T>::NotMinted);

            let deadline = <DeadlineOf<T>>::get(preferred_nft_id, &ad) //
                .ok_or(Error::<T>::NotExists)?;
            ensure!(deadline > height, Error::<T>::Deadline);

            ensure!(!<Payout<T>>::contains_key(&ad, &visitor), Error::<T>::Paid);

            // 1. get slot, check current ad
            let mut slot = <SlotOf<T>>::get(preferred_nft_id).ok_or(Error::<T>::NotExists)?;
            ensure!(slot.ad == ad, Error::<T>::Underbid);

            // 2. scoring visitor

            let mut socring = 5i32;

            let tags = T::Tags::tags_of(&ad);
            let personas = T::Tags::personas_of(&visitor);
            let length = personas.len();
            for (tag, score) in personas {
                let delta = if tags.contains_key(&tag) {
                    score.saturating_mul(10)
                } else {
                    score
                };
                socring.saturating_accrue(delta);
            }

            socring /= length.saturating_mul(10).saturating_add(1) as i32;

            if socring < 0 {
                socring = 0;
            }

            let socring = socring as u32;

            let amount = T::PayoutBase::get().saturating_mul(socring.into());

            if slot.tokens < amount {
                // if tokens is not enough, swap tokens

                // swap 10% of current budget, at least cover current payout
                Self::swap_by_10percent(kol, &meta, &nft_meta, &mut slot, amount)?;

                <SlotOf<T>>::insert(preferred_nft_id, &slot);
            }

            ensure!(slot.tokens >= amount, Error::<T>::InsufficientTokens);

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

                <T as Config>::Assets::transfer(slot.nft, &meta.pot, &referer, award, false)?;

                award
            } else {
                Zero::zero()
            };

            let reward = amount.saturating_sub(award);

            <T as Config>::Assets::transfer(slot.nft, &meta.pot, &account, reward, false)?;

            slot.tokens.saturating_reduce(amount);

            <SlotOf<T>>::insert(preferred_nft_id, &slot);

            <Payout<T>>::insert(&ad, &visitor, height);

            Self::deposit_event(Event::Paid(ad, slot.nft, visitor, reward, referer, award));

            // 5. drawback if advertiser does not have enough fees

            let controller = match Magic::<T>::controller(&who) {
                Some(c) => c,
                None => who,
            };

            let balance = <T as parami_did::Config>::Currency::free_balance(&controller);
            if balance - <T as parami_did::Config>::Currency::minimum_balance()
                < T::MinimumFeeBalance::get()
            {
                let _ = Self::drawback(preferred_nft_id, &slot);
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

        for (nft_id, ad, deadline) in <DeadlineOf<T>>::iter() {
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
                if slot.ad != ad {
                    continue;
                }

                read += 2;
                write += 4;
                let _ = Self::drawback(nft_id, &slot);

                amount += 1;
            }
        }

        let mut amount = 0;

        for (ad, endtime) in <EndtimeOf<T>>::iter() {
            read += 1;

            if amount >= 100 {
                break;
            }

            if endtime > now {
                continue;
            }

            read += 1;
            if let Some(slots) = <SlotsOf<T>>::get(ad) {
                if slots.len() > 0 {
                    continue;
                }
            }

            read += 1;
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

            let _ = <T as parami_did::Config>::Currency::transfer(
                &meta.pot,
                &creator.account,
                meta.remain,
                AllowDeath,
            );

            meta.remain = Zero::zero();

            write += 2;
            <Metadata<T>>::insert(ad, meta);
            <EndtimeOf<T>>::remove(ad);

            amount += 1;
        }

        Ok(T::DbWeight::get().reads_writes(read as Weight, write as Weight))
    }

    fn drawback(nft_id: NftIdOf<T>, slot: &SlotMetaOf<T>) -> Result<BalanceOf<T>, DispatchError> {
        let mut meta = <Metadata<T>>::get(slot.ad).ok_or(Error::<T>::NotExists)?;

        let amount = T::Swaps::token_in(
            meta.pot.clone(), //
            slot.nft,
            slot.tokens,
            One::one(),
            false,
        )?;

        meta.remain.saturating_accrue(slot.remain);
        meta.remain.saturating_accrue(amount);

        <Metadata<T>>::insert(slot.ad, meta);

        <SlotOf<T>>::remove(nft_id);

        <SlotsOf<T>>::mutate(slot.ad, |maybe| {
            if let Some(slots) = maybe {
                slots.retain(|x| *x != nft_id);
            }
        });

        <DeadlineOf<T>>::remove(nft_id, slot.ad);

        Self::deposit_event(Event::End(nft_id, slot.ad, amount));

        Ok(amount)
    }

    fn ensure_owned(did: DidOf<T>, ad: HashOf<T>) -> Result<MetaOf<T>, DispatchError> {
        let meta = <Metadata<T>>::get(&ad).ok_or(Error::<T>::NotExists)?;
        ensure!(meta.creator == did, Error::<T>::NotOwned);

        Ok(meta)
    }

    fn swap_by_10percent(
        kol: DidOf<T>,
        meta: &MetaOf<T>,
        nft_meta: &NftMetaFor<T>,
        slot: &mut SlotMetaOf<T>,
        least: BalanceOf<T>,
    ) -> DispatchResult {
        // swap per 10%
        let amount = slot.budget / 10u32.into();
        let tokens = T::Swaps::quote_in(
            meta.pot.clone(),
            nft_meta.token_asset_id,
            amount,
            least,
            false,
        )?;

        slot.remain.saturating_reduce(amount);
        slot.tokens.saturating_accrue(tokens);

        Self::deposit_event(Event::SwapTriggered(slot.ad, kol, slot.remain));

        Ok(())
    }
}
