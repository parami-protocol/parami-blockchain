#![cfg_attr(not(feature = "std"), no_std)]

pub use constants::*;
pub use pallet::*;
pub use types::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

mod constants;
mod types;
mod utils;

use frame_support::{
    dispatch::{DispatchError, DispatchResultWithPostInfo},
    ensure,
    traits::{
        tokens::fungibles::Transfer, Currency, EnsureOrigin, ExistenceRequirement::KeepAlive,
        ReservableCurrency,
    },
    transactional,
    weights::PostDispatchInfo,
    PalletId,
};
use parami_did::DidMethodSpecId;
use parami_primitives::Balance;
use sp_runtime::{
    traits::{AccountIdConversion, One, Saturating, Verify},
    DispatchErrorWithPostInfo, FixedPointNumber, PerU16,
};
use sp_std::vec::Vec;
use utils::*;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type MomentOf<T> = <T as pallet_timestamp::Config>::Moment;
pub type AdvertiserOf<T> = Advertiser<MomentOf<T>, AccountIdOf<T>>;
pub type AdvertisementOf<T> = Advertisement<MomentOf<T>, AccountIdOf<T>>;
pub type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<AccountIdOf<T>>>::Balance;
pub type BalanceOfAsset<T> = <T as pallet_assets::Config>::Balance;
pub type ResultPost<T> = sp_std::result::Result<T, DispatchErrorWithPostInfo<PostDispatchInfo>>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + pallet_timestamp::Config<AccountId = parami_primitives::AccountId>
        + pallet_staking::Config
        + parami_did::Config
        + pallet_assets::Config
    {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency mechanism.
        type Currency: ReservableCurrency<<Self as frame_system::Config>::AccountId>;

        /// Required `origin` for updating configuration
        type ConfigOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    /// Name of tag
    #[pallet::storage]
    pub type Tags<T: Config> = StorageMap<_, Identity, TagType, Vec<u8>>;

    /// A Coefficient to calculate the decay of an user score
    #[pallet::storage]
    pub type TimeDecayCoefficient<T: Config> = StorageValue<_, PerU16, ValueQuery>;

    /// The rate of extra rewards according to staking.
    #[pallet::storage]
    pub type StakingRewardRate<T: Config> = StorageValue<_, PerU16, ValueQuery>;

    /// the sender of `payout` will take an extra reward
    #[pallet::storage]
    pub type ExtraReward<T: Config> = StorageValue<_, Balance, ValueQuery>;

    /// ad deposit
    #[pallet::storage]
    pub type AdDeposit<T: Config> = StorageValue<_, Balance, ValueQuery>;

    /// advertiser deposit
    #[pallet::storage]
    pub type AdvertiserDeposit<T: Config> = StorageValue<_, Balance, ValueQuery>;

    /// Next available ID.
    #[pallet::storage]
    pub type NextId<T: Config> = StorageValue<_, GlobalId, ValueQuery>;

    /// an index for advertisers
    #[pallet::storage]
    pub type Advertisers<T: Config> =
        StorageMap<_, Blake2_128Concat, DidMethodSpecId, AdvertiserOf<T>>;

    /// an index for querying did by AdvertiserId
    #[pallet::storage]
    pub type AdvertiserById<T: Config> = StorageMap<_, Twox64Concat, AdvertiserId, DidMethodSpecId>;

    /// an index for advertisements
    #[pallet::storage]
    pub type Advertisements<T: Config> =
        StorageDoubleMap<_, Twox64Concat, AdvertiserId, Twox64Concat, AdId, AdvertisementOf<T>>;

    /// an index to tag score by tag type for every user.
    #[pallet::storage]
    pub type UserTagScores<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        DidMethodSpecId,
        Identity,
        TagType,
        (TagScore, T::Moment),
        ValueQuery,
        TagScoreDefault<T>,
    >;

    /// an index for rewards. The secondary key: `(user_did, media_did)`
    #[pallet::storage]
    pub type Rewards<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        AdId,
        Blake2_128Concat,
        (DidMethodSpecId, DidMethodSpecId),
        (),
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// an advertiser was created. \[who, did, advertiserId\]
        CreatedAdvertiser(AccountIdOf<T>, DidMethodSpecId, AdvertiserId),
        /// an advertisement was created. \[did, advertiserId, adId\]
        CreatedAd(DidMethodSpecId, AdvertiserId, AdId),
        AdReward(AdvertiserId, AdId, Balance),
    }

    #[pallet::error]
    pub enum Error<T> {
        SomethingTerribleHappened,
        /// The DID does not exist.
        DIDNotExists,
        /// Id overflow.
        NoAvailableId,
        /// Cannot find the advertiser.
        AdvertiserNotExists,
        /// Invalid Tag Coefficient Count
        InvalidTagCoefficientCount,
        /// Invalid Tag Type
        InvalidTagType,
        /// Duplicated Tag Type
        DuplicatedTagType,
        AdvertisementNotExists,
        NoPermission,
        ObsoletedDID,
        InvalidTagScoreDeltaLen,
        AdPaymentExpired,
        TagScoreDeltaOutOfRange,
        DuplicatedReward,
        TooEarlyToRedeem,
        AdvertiserExists,
        VecTooLong,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight((1_000, DispatchClass::Operational))]
        #[transactional]
        pub fn update_tag_name(
            origin: OriginFor<T>,
            tag_type: TagType,
            name: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            T::ConfigOrigin::ensure_origin(origin)?;
            ensure!(
                (tag_type as usize) < MAX_TAG_COUNT,
                Error::<T>::InvalidTagType
            );
            ensure!(name.len() > 1000, Error::<T>::VecTooLong);
            <Tags<T>>::insert(tag_type, name);
            Ok(().into())
        }

        #[pallet::weight((1_000, DispatchClass::Operational))]
        #[transactional]
        pub fn update_time_decay_coefficient(
            origin: OriginFor<T>,
            #[pallet::compact] coefficient: PerU16,
        ) -> DispatchResultWithPostInfo {
            T::ConfigOrigin::ensure_origin(origin)?;
            <TimeDecayCoefficient<T>>::put(coefficient);
            Ok(().into())
        }

        #[pallet::weight((1_000, DispatchClass::Operational))]
        #[transactional]
        pub fn update_extra_reward(
            origin: OriginFor<T>,
            #[pallet::compact] extra_reward: Balance,
        ) -> DispatchResultWithPostInfo {
            T::ConfigOrigin::ensure_origin(origin)?;
            <ExtraReward<T>>::put(extra_reward);
            Ok(().into())
        }

        #[pallet::weight(1_000_000_000)]
        #[transactional]
        pub fn create_advertiser(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            #[pallet::compact] reward_pool: Balance,
        ) -> DispatchResultWithPostInfo {
            let who: AccountIdOf<T> = ensure_signed(origin)?;
            let did: DidMethodSpecId = Self::ensure_did(&who)?;
            ensure!(
                <Advertisers<T>>::get(&did).is_none(),
                Error::<T>::AdvertiserExists
            );

            let advertiser_id = Self::inc_id()?;
            let (deposit_account, reward_pool_account) = Self::ad_accounts(advertiser_id);

            // active accounts
            <T as pallet::Config>::Currency::transfer(
                &who,
                &deposit_account,
                <T as pallet::Config>::Currency::minimum_balance(),
                KeepAlive,
            )?;
            <T as pallet::Config>::Currency::transfer(
                &who,
                &reward_pool_account,
                <T as pallet::Config>::Currency::minimum_balance(),
                KeepAlive,
            )?;

            let deposit = <AdvertiserDeposit<T>>::get();

            <pallet_assets::Pallet<T> as Transfer<AccountIdOf<T>>>::transfer(
                asset_id,
                &who,
                &deposit_account,
                s!(deposit),
                true,
            )?;
            <pallet_assets::Pallet<T> as Transfer<AccountIdOf<T>>>::transfer(
                asset_id,
                &who,
                &reward_pool_account,
                s!(reward_pool),
                true,
            )?;
            // <T as Config>::Currency::transfer(&who, &deposit_account, s!(deposit), KeepAlive)?;
            // <T as Config>::Currency::transfer(
            //     &who,
            //     &reward_pool_account,
            //     s!(reward_pool),
            //     KeepAlive,
            // )?;

            let a = Advertiser {
                created_time: now::<T>(),
                advertiser_id,
                deposit,
                deposit_account,
                reward_pool_account,
            };
            <Advertisers<T>>::insert(did, a);
            <AdvertiserById<T>>::insert(advertiser_id, did);
            Self::deposit_event(Event::CreatedAdvertiser(who, did, advertiser_id));
            Ok(().into())
        }

        #[pallet::weight(1_000_000_000)]
        #[transactional]
        pub fn create_ad(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            signer: AccountIdOf<T>,
            tag_coefficients: Vec<(TagType, TagCoefficient)>,
            media_reward_rate: PerU16,
            metadata: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let who: AccountIdOf<T> = ensure_signed(origin)?;

            ensure!(
                tag_coefficients.len() <= MAX_TAG_COUNT,
                Error::<T>::InvalidTagCoefficientCount
            );
            ensure!(
                !tag_coefficients.is_empty(),
                Error::<T>::InvalidTagCoefficientCount
            );

            for (tag_type, _) in &tag_coefficients {
                ensure!(*tag_type < MAX_TAG_TYPE_COUNT, Error::<T>::InvalidTagType);
                let mut count = 0;
                tag_coefficients.iter().for_each(|(t, _)| {
                    if tag_type == t {
                        count += 1;
                    }
                });
                ensure!(count == 1, Error::<T>::DuplicatedTagType);
            }

            let did: DidMethodSpecId = Self::ensure_did(&who)?;
            let ad_id = Self::inc_id()?;
            let advertiser = <Advertisers<T>>::get(&did).ok_or(Error::<T>::AdvertiserNotExists)?;
            let deposit = <AdDeposit<T>>::get();

            <pallet_assets::Pallet<T> as Transfer<AccountIdOf<T>>>::transfer(
                asset_id,
                &who,
                &advertiser.deposit_account,
                s!(deposit),
                true,
            )?;
            // <T as Config>::Currency::transfer(
            //     &who,
            //     &advertiser.deposit_account,
            //     s!(deposit),
            //     KeepAlive,
            // )?;
            // <T as Config>::Currency::reserve(&advertiser.deposit_account, s!(deposit))?;

            let ad = Advertisement {
                created_time: now::<T>(),
                deposit,
                tag_coefficients,
                signer,
                media_reward_rate,
                metadata,
            };
            <Advertisements<T>>::insert(advertiser.advertiser_id, ad_id, ad);
            Self::deposit_event(Event::CreatedAd(did, advertiser.advertiser_id, ad_id));
            Ok(().into())
        }

        /// advertiser pays some AD3 to user.
        #[pallet::weight(1_000_000_000)]
        #[transactional]
        pub fn ad_payout(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            ad_id: AdId,
            user_did: DidMethodSpecId,
            media_did: DidMethodSpecId,
            tag_score_delta: Vec<TagScore>,
        ) -> DispatchResultWithPostInfo {
            let advertiser = ensure_signed(origin)?;
            let advertiser_did: DidMethodSpecId = Self::ensure_did(&advertiser)?;

            let advertiser =
                <Advertisers<T>>::get(&advertiser_did).ok_or(Error::<T>::AdvertiserNotExists)?;
            let ad = <Advertisements<T>>::get(advertiser.advertiser_id, ad_id)
                .ok_or(Error::<T>::AdvertisementNotExists)?;
            let user = Self::lookup_index(user_did)?;
            let media = Self::lookup_index(media_did)?;

            ensure!(
                tag_score_delta.len() == ad.tag_coefficients.len(),
                Error::<T>::InvalidTagScoreDeltaLen
            );
            ensure!(
                <Rewards<T>>::get(ad_id, (user_did, media_did)).is_none(),
                Error::<T>::DuplicatedReward
            );

            let (reward, reward_media, reward_user) =
                calc_reward::<T>(&ad, &user_did, &user, &media, Some(&tag_score_delta))?;

            <pallet_assets::Pallet<T> as Transfer<AccountIdOf<T>>>::transfer(
                asset_id,
                &advertiser.reward_pool_account,
                &user,
                s!(reward_user),
                true,
            )?;
            <pallet_assets::Pallet<T> as Transfer<AccountIdOf<T>>>::transfer(
                asset_id,
                &advertiser.reward_pool_account,
                &media,
                s!(reward_media),
                true,
            )?;
            // <T as Config>::Currency::transfer(
            //     &advertiser.reward_pool_account,
            //     &user,
            //     s!(reward_user),
            //     KeepAlive,
            // )?;
            // <T as Config>::Currency::transfer(
            //     &advertiser.reward_pool_account,
            //     &media,
            //     s!(reward_media),
            //     KeepAlive,
            // )?;

            <Rewards<T>>::insert(ad_id, (user_did, media_did), ());
            Self::deposit_event(Event::AdReward(advertiser.advertiser_id, ad_id, reward));
            Ok(().into())
        }

        /// If advertiser fails to pay to user and media, everyone can trigger
        /// the process of payment.
        /// For the sake of fairness, the extrinsic sender will gain some extra AD3.
        #[pallet::weight(1_000_000_000)]
        #[transactional]
        pub fn payout(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            signature: Vec<u8>,
            advertiser_did: DidMethodSpecId,
            ad_id: AdId,
            user_did: DidMethodSpecId,
            media_did: DidMethodSpecId,
            timestamp: T::Moment,
        ) -> DispatchResultWithPostInfo {
            let sender: AccountIdOf<T> = ensure_signed(origin)?;

            let advertiser =
                <Advertisers<T>>::get(&advertiser_did).ok_or(Error::<T>::AdvertiserNotExists)?;
            let ad = <Advertisements<T>>::get(advertiser.advertiser_id, ad_id)
                .ok_or(Error::<T>::AdvertisementNotExists)?;
            let user = Self::lookup_index(user_did)?;
            let media = Self::lookup_index(media_did)?;

            let signature = sr25519_signature(&signature)?;
            let deadline =
                timestamp.saturating_add(s!(ADVERTISER_PAYMENT_WINDOW + USER_PAYMENT_WINDOW));
            let advertiser_payment_deadline =
                timestamp.saturating_add(s!(ADVERTISER_PAYMENT_WINDOW));

            // check timestamp
            let now = now::<T>();
            ensure!(now <= deadline, Error::<T>::AdPaymentExpired);
            ensure!(
                now > advertiser_payment_deadline,
                Error::<T>::TooEarlyToRedeem
            );

            let data =
                codec::Encode::encode(&(user_did, media_did, advertiser_did, timestamp, ad_id));
            ensure!(
                signature.verify(&data[..], &ad.signer),
                Error::<T>::NoPermission
            );

            ensure!(
                <Rewards<T>>::get(ad_id, (user_did, media_did)).is_none(),
                Error::<T>::DuplicatedReward
            );
            let (reward, reward_media, reward_user) =
                calc_reward::<T>(&ad, &user_did, &user, &media, None)?;

            let mut free: Balance = s!(free_balance::<T>(
                asset_id,
                advertiser.reward_pool_account.clone()
            ));
            if free > reward_user {
                <pallet_assets::Pallet<T> as Transfer<AccountIdOf<T>>>::transfer(
                    asset_id,
                    &advertiser.reward_pool_account,
                    &user,
                    s!(reward_user),
                    true,
                )?;
                // <T as Config>::Currency::transfer(
                //     &advertiser.reward_pool_account,
                //     &user,
                //     s!(reward_user),
                //     KeepAlive,
                // )?;
                free = free.saturating_sub(reward_user);
            } else {
                <pallet_assets::Pallet<T> as Transfer<AccountIdOf<T>>>::transfer(
                    asset_id,
                    &advertiser.deposit_account,
                    &user,
                    s!(reward_user),
                    true,
                )?;
                // <T as Config>::Currency::transfer(
                //     &advertiser.deposit_account,
                //     &user,
                //     s!(reward_user),
                //     KeepAlive,
                // )?;
            }

            if free > reward_media {
                <pallet_assets::Pallet<T> as Transfer<AccountIdOf<T>>>::transfer(
                    asset_id,
                    &advertiser.reward_pool_account,
                    &media,
                    s!(reward_media),
                    true,
                )?;
                // <T as Config>::Currency::transfer(
                //     &advertiser.reward_pool_account,
                //     &media,
                //     s!(reward_media),
                //     KeepAlive,
                // )?;
                free = free.saturating_sub(reward_media);
            } else {
                <pallet_assets::Pallet<T> as Transfer<AccountIdOf<T>>>::transfer(
                    asset_id,
                    &advertiser.deposit_account,
                    &media,
                    s!(reward_media),
                    true,
                )?;
                // <T as Config>::Currency::transfer(
                //     &advertiser.deposit_account,
                //     &media,
                //     s!(reward_media),
                //     KeepAlive,
                // )?;
            }

            let extra_reward = <ExtraReward<T>>::get();
            if free > extra_reward {
                <pallet_assets::Pallet<T> as Transfer<AccountIdOf<T>>>::transfer(
                    asset_id,
                    &advertiser.reward_pool_account,
                    &sender,
                    s!(extra_reward),
                    true,
                )?;
            // <T as Config>::Currency::transfer(
            //     &advertiser.reward_pool_account,
            //     &sender,
            //     s!(extra_reward),
            //     KeepAlive,
            // )?;
            } else {
                <pallet_assets::Pallet<T> as Transfer<AccountIdOf<T>>>::transfer(
                    asset_id,
                    &advertiser.deposit_account,
                    &sender,
                    s!(extra_reward),
                    true,
                )?;
                // <T as Config>::Currency::transfer(
                //     &advertiser.deposit_account,
                //     &sender,
                //     s!(extra_reward),
                //     KeepAlive,
                // )?;
            }

            <Rewards<T>>::insert(ad_id, (user_did, media_did), ());
            Self::deposit_event(Event::AdReward(advertiser.advertiser_id, ad_id, reward));
            Ok(().into())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub advertiser_deposit: Balance,
        pub ad_deposit: Balance,
        pub extra_reward: Balance,
        pub time_decay_coefficient: PerU16,
        pub staking_reward_rate: PerU16,
        pub tag_names: Vec<(TagType, Vec<u8>)>,
        pub _phantom: PhantomData<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            let mut tag_names = Vec::new();
            tag_names.push((0, b"type01".as_ref().into()));
            tag_names.push((1, b"type02".as_ref().into()));
            tag_names.push((3, b"type03".as_ref().into()));
            tag_names.push((4, b"type04".as_ref().into()));
            Self {
                advertiser_deposit: UNIT.saturating_mul(100),
                ad_deposit: UNIT.saturating_mul(100),
                extra_reward: UNIT.saturating_mul(3),
                staking_reward_rate: PerU16::from_percent(2),
                time_decay_coefficient: PerU16::from_percent(1),
                tag_names,
                _phantom: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <AdvertiserDeposit<T>>::put(self.advertiser_deposit);
            <AdDeposit<T>>::put(self.ad_deposit);
            <ExtraReward<T>>::put(self.extra_reward);
            <StakingRewardRate<T>>::put(self.staking_reward_rate);
            <TimeDecayCoefficient<T>>::put(self.time_decay_coefficient);
            for (tag, name) in &self.tag_names {
                <Tags<T>>::insert(tag, name);
            }
        }
    }
}

impl<T: Config> Pallet<T> {
    pub fn ensure_did(who: &AccountIdOf<T>) -> ResultPost<DidMethodSpecId> {
        let did: Option<DidMethodSpecId> = parami_did::Pallet::<T>::lookup_account(who.clone());
        ensure!(did.is_some(), Error::<T>::DIDNotExists);
        Ok(did.expect("Must be Some"))
    }

    fn lookup_index(did: DidMethodSpecId) -> ResultPost<AccountIdOf<T>> {
        let who: Option<AccountIdOf<T>> = parami_did::Pallet::<T>::lookup_index(did);
        ensure!(who.is_some(), Error::<T>::ObsoletedDID);
        Ok(who.expect("Must be Some"))
    }

    pub fn ad_accounts(id: AdvertiserId) -> (AccountIdOf<T>, AccountIdOf<T>) {
        let deposit = PalletId(*b"prm/ad/d");
        let reward_pool = PalletId(*b"prm/ad/r");
        (
            deposit.into_sub_account(id),
            reward_pool.into_sub_account(id),
        )
    }

    fn inc_id() -> Result<GlobalId, DispatchError> {
        <NextId<T>>::try_mutate(|id| -> Result<GlobalId, DispatchError> {
            let current_id = *id;
            *id = id
                .checked_add(GlobalId::one())
                .ok_or(Error::<T>::NoAvailableId)?;
            Ok(current_id)
        })
    }
}
