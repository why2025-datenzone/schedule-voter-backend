use std::{collections::{HashMap, HashSet}, marker::PhantomData};

use bitvec::{bitvec, vec::BitVec};
use entity::vote;

use crate::{EventConflictItem, SubmissionConflictQueryItem, SubmissionSimilarityItem};

use itertools::Itertools;

pub trait VoteMetric {
    fn process_vote(self: &mut Self, vote: Vec<Vec<String>>, mapper: impl CodeIndexMapper, pi: impl PairsIterator);
    fn finalize(self: Self) -> impl IntoIterator<Item=f64>;
    fn with_size(maxindex: usize, maxpairs: usize) -> Self;
    fn get_columns() -> impl IntoIterator<Item=entity::vote::Column>;
}

pub trait VoteSelector {
    fn get_columns() -> impl IntoIterator<Item=entity::vote::Column>;
    fn get_votes(dbres: &[Vec<String>]) -> impl IntoIterator<Item=&String>;
}

pub struct UpVoteSelector {

}

impl VoteSelector for UpVoteSelector {
    fn get_columns() -> impl IntoIterator<Item=entity::vote::Column> {
        return [vote::Column::Up];
    }

    fn get_votes(dbres: &[Vec<String>]) -> impl IntoIterator<Item=&String> {
        return dbres[0].iter();
    }
}

pub struct ExpandedVoteSelector {

}

impl VoteSelector for ExpandedVoteSelector {
    fn get_columns() -> impl IntoIterator<Item=entity::vote::Column> {
        return [vote::Column::Expanded];
    }

    fn get_votes(dbres: &[Vec<String>]) -> impl IntoIterator<Item=&String> {
        return dbres[0].iter();
    }
}

pub struct ExpandedWithoutDownVoteSelector {

}

impl VoteSelector for ExpandedWithoutDownVoteSelector {
    fn get_columns() -> impl IntoIterator<Item=entity::vote::Column> {
        return [vote::Column::Expanded, vote::Column::Down];
    }

    fn get_votes(dbres: &[Vec<String>]) -> impl IntoIterator<Item=&String> {
        let expanded = &dbres[0];
        let down = &dbres[1];
        // let [expanded, down]: &[Vec<String>;2] = dbres.try_into().unwrap();
        let downset: HashSet<&String> = HashSet::from_iter(down.into_iter());
        return expanded.into_iter().filter(move |s| !downset.contains(*s));
    }
}

pub trait PairsIterator {
    fn pairs(self: &Self) -> impl IntoIterator<Item=&(usize, usize)>;
}

pub trait CodeIndexMapper {
    fn vote_to_index(self: &Self, code: &String) -> Option<usize>;
}

#[derive(Default,Clone)]
struct CosineVoteEntry {
    dotprod: i32,
    a: i32,
    b: i32,
}

impl CosineVoteEntry {
    fn ratio(self: &Self) -> f64 {
        if (self.a == 0) && (self.b == 0) {
            return 0.0;
        } else {
            return (self.dotprod as f64)/((self.a as f64).sqrt()*((self.b as f64).sqrt()))
        }
    }
}

pub struct CosineVoteMetric {
    vote_pair: Vec<CosineVoteEntry>,
    maxindex: usize,
}

fn cosine_weight(up: bool, expanded: bool, down: bool) -> i32 {
    if up { 4 } else if expanded { 1 } else if down { -4 } else { 0 }
}

impl VoteMetric for CosineVoteMetric {
    fn process_vote(self: &mut Self, vote: Vec<Vec<String>>, mapper: impl CodeIndexMapper, pi: impl PairsIterator) {
        let vote_up = vote_to_bitvec(&mapper,&vote[0], self.maxindex);
        let vote_ex = vote_to_bitvec(&mapper,&vote[1], self.maxindex);
        let vote_down = vote_to_bitvec(&mapper,&vote[2], self.maxindex);
        let vote_view = vote_to_bitvec(&mapper,&vote[3], self.maxindex);
        for (i,(a,b)) in pi.pairs().into_iter().enumerate() {
            if vote_view[*a] && vote_view[*b] {
                let entry = &mut self.vote_pair[i];
                let weight_a = cosine_weight(vote_up[*a], vote_ex[*a], vote_down[*a]);
                let weight_b = cosine_weight(vote_up[*b], vote_ex[*b], vote_down[*b]);
                entry.a += weight_a*weight_a;
                entry.b += weight_b*weight_b;
                entry.dotprod += weight_a * weight_b;
            }
        }
    }

    fn finalize(self: Self) -> impl IntoIterator<Item=f64> {
        self.vote_pair.into_iter().map(|x| x.ratio())
    }

    fn with_size(maxindex: usize, maxpairs: usize) -> Self {
        Self {
            vote_pair: vec![CosineVoteEntry::default();maxpairs],
            maxindex: maxindex,
        }
    }

    fn get_columns() -> impl IntoIterator<Item=entity::vote::Column> {
        return [vote::Column::Up, vote::Column::Expanded, vote::Column::Down, vote::Column::Shown]
    }
}

#[derive(Default,Clone)]
struct LiftVoteEntry {
    a: u32,
    b: u32,
    joint: u32,
    views: u32,
}

impl LiftVoteEntry {
    fn ratio(self: &Self) -> f64 {
        if (self.a == 0) || (self.b == 0) {
            return 0.0;
        } else {
            return ((self.joint as f64)*(self.views as f64))/((self.a as f64)*(self.b as f64));
        }
    }
}


pub struct LiftVoteMetric<T> {
    vote_pair: Vec<LiftVoteEntry>,
    maxindex: usize,
    vote_type: PhantomData<T>,
    // columns: Vec<entity::vote::Column>,
}

impl<T: VoteSelector> VoteMetric for LiftVoteMetric<T> {
    fn finalize(self: Self) -> impl IntoIterator<Item=f64> {
        let ratio_iterator = self.vote_pair.into_iter().map(|x| x.ratio());
        return ratio_iterator
    }

    fn with_size(maxindex: usize, pairs: usize) -> Self {
        Self { 
            vote_pair: vec![LiftVoteEntry::default(); pairs], 
            maxindex: maxindex,
            // columns: T::get_columns().into_iter().collect(),
            vote_type: PhantomData 
        }
    }

    fn get_columns() -> impl IntoIterator<Item=entity::vote::Column> {
        T::get_columns().into_iter().chain([entity::vote::Column::Shown])
    }
    
    fn process_vote(self: &mut Self, vote: Vec<Vec<String>>, mapper: impl CodeIndexMapper, pi: impl PairsIterator) {
        let converted_votes = T::get_votes(&vote[0..(vote.len()-1)]);
        let views = &vote[vote.len()-1];

        let votevec = vote_to_bitvec(&mapper, converted_votes, self.maxindex);
        let viewvec = vote_to_bitvec(&mapper, views, self.maxindex);

        for (i,(a,b)) in pi.pairs().into_iter().enumerate() {
            
            let votea = votevec[*a];
            let voteb = votevec[*b];
            let voteboth = votea && voteb;
            let viewboth = viewvec[*a] && viewvec[*b];
            let entry = &mut self.vote_pair[i];
            if viewboth {
                entry.views += 1;
                if votea {
                    entry.a += 1;
                }
                if voteb {
                    entry.b += 1;
                }
                if voteboth {
                    entry.joint += 1;
                }
            }
        }
    }
}

fn vote_to_bitvec<'a>(mapper: &impl CodeIndexMapper, votes: impl IntoIterator<Item = &'a String>, maxindex: usize) -> BitVec {
    let mut votevec = bitvec![0;maxindex];
    for v in votes.into_iter().filter_map(|v| mapper.vote_to_index(v)) {
        votevec.set(v, true);
    }
    return votevec;
}


#[derive(Default,Clone)]
pub struct WeightedSupportVoteEntry {
    numerator: f64,
    denominator: u32,
}

impl WeightedSupportVoteEntry {
    fn ratio(self: &Self) -> f64 {
        if self.denominator == 0 {
            return 0.0;
        } else {
            return (self.numerator)/(self.denominator as f64);
        }
    }
}

#[derive(Default,Clone)]
struct JaccardSupportVoteEntry {
    numerator: u32,
    denominator: u32,
}

impl JaccardSupportVoteEntry {
    fn ratio(self: &Self) -> f64 {
        if self.denominator == 0 {
            return 0.0;
        } else {
            return (self.numerator as f64)/(self.denominator as f64);
        }
    }
}

pub struct SupportVoteMetric<T> {
    vote_pair: Vec<JaccardSupportVoteEntry>,
    maxindex: usize,
    vote_type: PhantomData<T>,
    // columns: Vec<entity::vote::Column>,
}

impl<T: VoteSelector> SupportVoteMetric<T> {
    fn total_columns() -> Vec<entity::vote::Column> {
        let mut vote_columns = T::get_columns().into_iter().collect::<Vec<_>>();
        vote_columns.push(entity::vote::Column::Shown);
        return vote_columns;
    }
}

impl<T: VoteSelector> WeightedSupportMetric<T> {
    fn total_columns() -> Vec<entity::vote::Column> {
        let mut vote_columns = T::get_columns().into_iter().collect::<Vec<_>>();
        vote_columns.push(entity::vote::Column::Shown);
        return vote_columns;
    }
}

impl<T: VoteSelector> VoteMetric for SupportVoteMetric<T> {
    fn finalize(self: Self) -> impl IntoIterator<Item=f64> {
        let ratio_iterator = self.vote_pair.into_iter().map(|x| x.ratio());
        return ratio_iterator
    }

    fn with_size(maxindex: usize, pairs: usize) -> Self {
        Self { 
            vote_pair: vec![JaccardSupportVoteEntry::default(); pairs], 
            maxindex: maxindex,
            // columns: Self::total_columns(),
            vote_type: PhantomData 
        }
    }

    fn get_columns() -> impl IntoIterator<Item=entity::vote::Column> {
        Self::total_columns()
    }
    
    fn process_vote(self: &mut Self, vote: Vec<Vec<String>>, mapper: impl CodeIndexMapper, pi: impl PairsIterator) {
        let converted_votes = T::get_votes(&vote[0..(vote.len()-1)]);
        let views = &vote[vote.len()-1];

        let votevec = vote_to_bitvec(&mapper, converted_votes, self.maxindex);
        let viewvec = vote_to_bitvec(&mapper, views, self.maxindex);

        for (i,(a,b)) in pi.pairs().into_iter().enumerate() {
            let voteboth = votevec[*a] && votevec[*b];
            let viewboth = viewvec[*a] && viewvec[*b];
            let entry = &mut self.vote_pair[i];
            if viewboth {
                entry.denominator += 1;
            }
            if voteboth {
                entry.numerator += 1;
            }
        }
    }
}

pub struct WeightedSupportMetric<T> {
    vote_pair: Vec<WeightedSupportVoteEntry>,
    maxindex: usize,
    vote_type: PhantomData<T>,
    // columns: Vec<entity::vote::Column>,
}

impl<T: VoteSelector> VoteMetric for WeightedSupportMetric<T> {
    fn finalize(self: Self) -> impl IntoIterator<Item=f64> {
        let ratio_iterator = self.vote_pair.into_iter().map(|x| x.ratio());
        return ratio_iterator
    }

    fn with_size(maxindex: usize, pairs: usize) -> Self {
        Self { 
            vote_pair: vec![WeightedSupportVoteEntry::default(); pairs], 
            maxindex: maxindex,
            // columns: Self::total_columns(),
            vote_type: PhantomData 
        }
    }

    fn get_columns() -> impl IntoIterator<Item=entity::vote::Column> {
        Self::total_columns()
    }
    
    fn process_vote(self: &mut Self, vote: Vec<Vec<String>>, mapper: impl CodeIndexMapper, pi: impl PairsIterator) {
                let converted_votes = T::get_votes(&vote[0..(vote.len()-1)]);
        let views = &vote[vote.len()-1];

        let mut votevec = bitvec![0;self.maxindex];

        let mut totalvotes = 0;
        for v in converted_votes.into_iter().filter_map(|v| mapper.vote_to_index(v)) {
            votevec.set(v, true);
            totalvotes += 1;
        }

        let viewvec = vote_to_bitvec(&mapper, views, self.maxindex);

        for (i,(a,b)) in pi.pairs().into_iter().enumerate() {
            let voteboth = votevec[*a] && votevec[*b];
            let viewboth = viewvec[*a] && viewvec[*b];
            let entry = &mut self.vote_pair[i];
            if viewboth {
                entry.denominator += 1;
            }
            if voteboth {
                entry.numerator += 1.0/(totalvotes as f64);
            }
        }
    }

}

pub struct JaccardVoteMetric<T: VoteSelector> {
    vote_pair: Vec<JaccardSupportVoteEntry>,
    maxindex: usize,
    vote_type: PhantomData<T>,
    columns: Vec<entity::vote::Column>,
}

impl<T: VoteSelector> VoteMetric for JaccardVoteMetric<T> {
    fn process_vote(self: &mut Self, vote: Vec<Vec<String>>, mapper: impl CodeIndexMapper, pi: impl PairsIterator) {
        let converted_votes = T::get_votes(&vote[0..self.columns.len()]);
        let views = &vote[vote.len()-1];

        let votevec = vote_to_bitvec(&mapper, converted_votes, self.maxindex);
        let viewvec = vote_to_bitvec(&mapper, views, self.maxindex);

        for (i,(a,b)) in pi.pairs().into_iter().enumerate() {
            let ain = votevec[*a];
            let bin = votevec[*b];
            let entry = &mut self.vote_pair[i];
            if (ain||bin) && viewvec[*a] && viewvec[*b] {
                entry.denominator += 1;
                if ain && bin {
                    entry.numerator += 1;
                }
            }
        }
    }

    fn finalize(self: Self) -> impl IntoIterator<Item=f64> {
        let ratio_iterator = self.vote_pair.into_iter().map(|x| x.ratio());
        return ratio_iterator
    }

    fn with_size(maxindex: usize, pairs: usize) -> Self {
        Self { 
            vote_pair: vec![JaccardSupportVoteEntry::default(); pairs], 
            maxindex: maxindex,
            columns: T::get_columns().into_iter().collect(),
            vote_type: PhantomData 
        }
    }

    fn get_columns() -> impl IntoIterator<Item=entity::vote::Column> {
        T::get_columns().into_iter().chain([entity::vote::Column::Shown])
    }
}

pub struct SimilarityStruct<T: VoteMetric> {
    fullcodes: Vec<(String,String)>,
    pairs: Vec<(usize, usize)>,
    all_conflict_fullcodes: HashMap<String,usize>,
    metrics: T,
}

impl PairsIterator for &Vec<(usize,usize)> {
    fn pairs(self: &Self) -> impl IntoIterator<Item=&(usize, usize)> {
        self.iter()
    }
}

impl CodeIndexMapper for &HashMap<String,usize> {
    fn vote_to_index(self: &Self, code: &String) -> std::option::Option<usize> {
        self.get(code).copied()
    }
}

impl<T: VoteMetric> SimilarityStruct<T> {
    fn internal_constructor(pairs: impl IntoIterator<Item=(String,String)>) -> Self {
        let mut all_conflict_fullcodes: HashMap<String, usize> = HashMap::new();
        let mut fullcodes = Vec::new();
        let mut index:usize = 0;
        for (a,b) in pairs.into_iter() {
            if !all_conflict_fullcodes.contains_key(&a) {
                all_conflict_fullcodes.insert(a.clone(), index);
                index +=1;
            }
            if !all_conflict_fullcodes.contains_key(&b) {
                all_conflict_fullcodes.insert(b.clone(), index);
                index +=1;
            }
            fullcodes.push((a.clone(),b.clone()));
        }
        
        let maxsize = index;

        let pairs = fullcodes
            .iter()
            .map(
                |(a,b)| 
                (
                    *all_conflict_fullcodes.get(a).unwrap(), 
                    *all_conflict_fullcodes.get(b).unwrap()
                )
            )
            .collect::<Vec<(usize, usize)>>();
        let metrics = VoteMetric::with_size(
            maxsize, 
            pairs.len()
            );

        return Self { 
            fullcodes: fullcodes, 
            all_conflict_fullcodes: all_conflict_fullcodes, 
            metrics: metrics,
            pairs: pairs,
        }
    }

    fn prepare_result(self: Self, limit: usize) -> std::iter::Take<std::vec::IntoIter<((String, String), f64)>> {
        let metric_iterator = self.metrics.finalize();
        let res = Iterator::zip(
            self.fullcodes.into_iter(), 
            metric_iterator)
            .sorted_by(|(_, a),(_, b)| {
                b.total_cmp(&a)
            })
            .take(limit as usize);
                    res
    }
}

pub trait Similarity {
    fn get_columns(self: &Self) -> Vec<vote::Column>;
    fn process_votes(self: &mut Self, votes: Vec<Vec<Vec<String>>>);
    fn from_single_submission(submission_id: String, other_submissions: Vec<String>) -> Self where Self: Sized;
    fn from_query_results(submission_conflicts: Vec<SubmissionConflictQueryItem>) -> Self where Self: Sized;
    fn get_item_results(self: Self, limit: usize) -> Vec<SubmissionSimilarityItem>;
    fn get_results(self: Self, limit: usize) -> Vec<EventConflictItem>;
}

impl<T: VoteMetric> Similarity for SimilarityStruct<T> {
    fn get_columns(self: &Self) -> Vec<vote::Column> {
        return <T as VoteMetric>::get_columns().into_iter().collect();
    }

    fn process_votes(self: &mut Self, votes: Vec<Vec<Vec<String>>>) {
        for vote in votes.into_iter() {
            self.metrics.process_vote(vote, &self.all_conflict_fullcodes, &self.pairs);
        }
    }

    fn from_single_submission(submission_id: String, other_submissions: Vec<String>) -> Self {
        Self::internal_constructor(other_submissions.into_iter().map(|o| (submission_id.clone(), o)))
    }

    fn from_query_results(submission_conflicts: Vec<SubmissionConflictQueryItem>) -> Self {
        Self::internal_constructor(submission_conflicts.into_iter().map(|sc| (sc.fullcode_a, sc.fullcode_b)))
    }

    fn get_item_results(self: Self, limit: usize) -> Vec<SubmissionSimilarityItem> {
        let res = self.prepare_result(limit);
        return res.map(
            |((_,code),m)| 
            SubmissionSimilarityItem {
                id: code,
                metric: m,
            })
            .collect::<Vec<_>>();
    }

    fn get_results(self: Self, limit: usize) -> Vec<EventConflictItem> {
        let res = self.prepare_result(limit);
        return res.map(
                |((a,b),m)| 
                EventConflictItem {
                    a,
                    b,
                    correlation: m,
                })
            .collect::<Vec<EventConflictItem>>();
    }

    
}