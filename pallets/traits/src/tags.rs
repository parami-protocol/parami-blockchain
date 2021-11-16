use codec::MaxEncodedLen;
use frame_support::Parameter;
use sp_runtime::{
    traits::{MaybeSerializeDeserialize, Member},
    DispatchResult,
};
use sp_std::prelude::*;

pub trait Tags {
    type DecentralizedId: Parameter + Member + MaybeSerializeDeserialize + MaxEncodedLen;

    type Hash: Parameter + Member + MaybeSerializeDeserialize + MaxEncodedLen;

    /// Get hashed value of a tag
    ///
    /// # Arguments
    ///
    /// * `tag` - Tag to be hashed
    ///
    /// # Returns
    ///
    /// Hashed value of the tag
    fn key<K: AsRef<Vec<u8>>>(tag: K) -> Vec<u8>;

    /// Determine if a tag is valid
    ///
    /// # Arguments
    ///
    /// * `tag` - Tag to be checked
    ///
    /// # Returns
    ///
    /// `true` if the tag is valid, `false` otherwise
    fn exists<K: AsRef<Vec<u8>>>(tag: K) -> bool;

    /// Get all tags of an advertisement
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the advertisement
    ///
    /// # Returns
    ///
    /// List of tags of the advertisement
    fn tags_of(id: &Self::Hash) -> Vec<Vec<u8>>;

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
    fn add_tag(id: &Self::Hash, tag: Vec<u8>) -> DispatchResult;

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
    fn del_tag<K: AsRef<Vec<u8>>>(id: &Self::Hash, tag: K) -> DispatchResult;

    /// Clear tags of an advertisement
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the advertisement
    ///
    /// # Returns
    ///
    /// `Ok` if the score is updated, `Err` otherwise
    fn clr_tag(id: &Self::Hash) -> DispatchResult;

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
    fn has_tag<K: AsRef<Vec<u8>>>(id: &Self::Hash, tag: K) -> bool;

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
    fn personas_of(did: &Self::DecentralizedId) -> Vec<(Vec<u8>, i32)>;

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
    fn get_score<K: AsRef<Vec<u8>>>(did: &Self::DecentralizedId, tag: K) -> i32;

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
    fn influence<K: AsRef<Vec<u8>>>(
        did: &Self::DecentralizedId,
        tag: K,
        delta: i32,
    ) -> DispatchResult;

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
    fn influences_of(kol: &Self::DecentralizedId) -> Vec<(Vec<u8>, i32)>;

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
    fn get_influence<K: AsRef<Vec<u8>>>(kol: &Self::DecentralizedId, tag: K) -> i32;

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
    fn impact<K: AsRef<Vec<u8>>>(
        kol: &Self::DecentralizedId,
        tag: K, //
        delta: i32,
    ) -> DispatchResult;
}
