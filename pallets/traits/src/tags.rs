use sp_runtime::DispatchResult;
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

type Tag = Vec<u8>;

pub trait Tags<Hash, AdvertisementId, DecentralizedId> {
    /// Get hashed value of a tag
    ///
    /// # Arguments
    ///
    /// * `tag` - Tag to be hashed
    ///
    /// # Returns
    ///
    /// Hashed value of the tag
    fn key<K: AsRef<Tag>>(tag: K) -> Hash;

    /// Determine if a tag is valid
    ///
    /// # Arguments
    ///
    /// * `tag` - Tag to be checked
    ///
    /// # Returns
    ///
    /// `true` if the tag is valid, `false` otherwise
    fn exists<K: AsRef<Tag>>(tag: K) -> bool;

    /// Get all tags of an advertisement
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the advertisement
    ///
    /// # Returns
    ///
    /// List of tags of the advertisement
    fn tags_of(id: &AdvertisementId) -> BTreeMap<Hash, bool>;

    /// Add a tag to an advertisement
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the advertisement
    /// * `tag` - Tag to be add
    ///
    /// # Returns
    ///
    /// `Ok` if the score is updated, `Err` otherwise
    fn add_tag(id: &AdvertisementId, tag: Tag) -> DispatchResult;

    /// Remove a tag from an advertisement
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the advertisement
    /// * `tag` - Tag to be removed
    ///
    /// # Returns
    ///
    /// `Ok` if the score is updated, `Err` otherwise
    fn del_tag<K: AsRef<Tag>>(id: &AdvertisementId, tag: K) -> DispatchResult;

    /// Clear tags of an advertisement
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the advertisement
    ///
    /// # Returns
    ///
    /// `Ok` if the score is updated, `Err` otherwise
    fn clr_tag(id: &AdvertisementId) -> DispatchResult;

    /// Determine if an advertisement has a tag
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the advertisement
    /// * `tag` - Tag to be checked
    ///
    /// # Returns
    ///
    /// `true` if the advertisement has the tag, `false` otherwise
    fn has_tag<K: AsRef<Tag>>(id: &AdvertisementId, tag: K) -> bool;

    /// Get all tags and scores of a DID
    ///
    /// # Arguments
    ///
    /// * `did` - the DID
    ///
    /// # Returns
    ///
    /// (hashed, score)
    ///
    /// * `hashed` - hashed tags
    /// * `score` - score of the DID
    fn personas_of(did: &DecentralizedId) -> BTreeMap<Hash, i32>;

    /// Get a persona's score
    ///
    /// # Arguments
    ///
    /// * `did` - the DID
    /// * `tag` - Tag of the persona
    ///
    /// # Returns
    ///
    /// score of the persona
    fn get_score<K: AsRef<Tag>>(did: &DecentralizedId, tag: K) -> i32;

    /// Update score of a persona
    ///
    /// # Arguments
    ///
    /// * `did` - the DID
    /// * `tag` - Tag of the persona
    /// * `delta` - Score delta of the persona
    ///
    /// # Returns
    ///
    /// `Ok` if the score is updated, `Err` otherwise
    fn influence<K: AsRef<Tag>>(did: &DecentralizedId, tag: K, delta: i32) -> DispatchResult;

    /// Get all tags and scores of a KOL
    ///
    /// # Arguments
    ///
    /// * `kol` - the DID of the KOL
    ///
    /// # Returns
    ///
    /// (hashed, score)
    ///
    /// * `hashed` - hashed tags
    /// * `score` - score of the KOL
    fn influences_of(kol: &DecentralizedId) -> BTreeMap<Hash, i32>;

    /// Get a KOL's score
    ///
    /// # Arguments
    ///
    /// * `kol` - the DID of the KOL
    /// * `tag` - Tag of the KOL
    ///
    /// # Returns
    ///
    /// score of the KOL
    fn get_influence<K: AsRef<Tag>>(kol: &DecentralizedId, tag: K) -> i32;

    /// Update score of a KOL
    ///
    /// # Arguments
    ///
    /// * `kol` - the DID of the KOL
    /// * `tag` - Tag of the KOL
    /// * `delta` - Score delta of the KOL
    ///
    /// # Returns
    ///
    /// `Ok` if the score is updated, `Err` otherwise
    fn impact<K: AsRef<Tag>>(
        kol: &DecentralizedId,
        tag: K, //
        delta: i32,
    ) -> DispatchResult;
}
