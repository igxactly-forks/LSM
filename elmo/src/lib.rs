﻿/*
    Copyright 2014-2015 Zumero, LLC

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

        http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
*/

#![feature(convert)]
#![feature(box_syntax)]
#![feature(associated_consts)]

extern crate misc;

use misc::endian;
use misc::bufndx;
use misc::varint;

extern crate bson;
use bson::BsonValue;

#[derive(Debug)]
// TODO do we really want this public?
pub enum Error {
    // TODO remove Misc
    Misc(&'static str),

    // TODO more detail within CorruptFile
    CorruptFile(&'static str),

    Bson(bson::Error),
    Io(std::io::Error),
    Utf8(std::str::Utf8Error),
    Whatever(Box<std::error::Error>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::Bson(ref err) => write!(f, "bson error: {}", err),
            Error::Io(ref err) => write!(f, "IO error: {}", err),
            Error::Utf8(ref err) => write!(f, "Utf8 error: {}", err),
            Error::Whatever(ref err) => write!(f, "Other error: {}", err),
            Error::Misc(s) => write!(f, "Misc error: {}", s),
            Error::CorruptFile(s) => write!(f, "Corrupt file: {}", s),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Bson(ref err) => std::error::Error::description(err),
            Error::Io(ref err) => std::error::Error::description(err),
            Error::Utf8(ref err) => std::error::Error::description(err),
            Error::Whatever(ref err) => std::error::Error::description(&**err),
            Error::Misc(s) => s,
            Error::CorruptFile(s) => s,
        }
    }

    // TODO cause
}

// TODO why is 'static needed here?  Doesn't this take ownership?
pub fn wrap_err<E: std::error::Error + 'static>(err: E) -> Error {
    Error::Whatever(box err)
}

impl From<bson::Error> for Error {
    fn from(err: bson::Error) -> Error {
        Error::Bson(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

// TODO not sure this is useful
impl From<Box<std::error::Error>> for Error {
    fn from(err: Box<std::error::Error>) -> Error {
        Error::Whatever(err)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Error {
        Error::Utf8(err)
    }
}

/*
impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(_err: std::sync::PoisonError<T>) -> Error {
        Error::Poisoned
    }
}

impl<'a, E: Error + 'a> From<E> for Error {
    fn from(err: E) -> Error {
        Error::Whatever(err)
    }
}
*/

pub type Result<T> = std::result::Result<T, Error>;

pub struct IndexInfo {
    pub db: String,
    pub coll: String,
    pub name: String,
    pub spec: BsonValue,
    pub options: BsonValue,
}

impl IndexInfo {
    pub fn full_collection_name(&self) -> String {
        format!("{}.{}", self.db, self.coll)
    }
}

pub type QueryKey = Vec<BsonValue>;

pub enum TextQueryTerm {
    Word(bool, String),
    Phrase(bool, String),
}

pub enum QueryBounds {
    EQ(QueryKey),
    Text(QueryKey,Vec<TextQueryTerm>),
    GT(QueryKey),
    GTE(QueryKey),
    LT(QueryKey),
    LTE(QueryKey),
    GT_LT(QueryKey, QueryKey),
    GT_LTE(QueryKey, QueryKey),
    GTE_LT(QueryKey, QueryKey),
    GTE_LTE(QueryKey, QueryKey),
}

pub struct QueryPlan {
    pub ndx: IndexInfo,
    pub bounds: QueryBounds,
}

// TODO the Item below should be a struct that contains both the BsonValue
// and also the score (and maybe pos).

pub trait StorageCollectionReader {
    fn iter<'a>(&'a self) -> &'a Iterator<Item=Result<BsonValue>>;
    fn get_total_keys_examined(&self) -> u64;
    fn get_total_docs_examined(&self) -> u64;
    // TODO more explain stuff here?
}

pub trait StorageReader {
    // TODO maybe these two should return an iterator
    // TODO maybe these two should accept params to limit the rows returned
    fn list_collections(&self) -> Result<Vec<(String, String, BsonValue)>>;
    fn list_indexes(&self) -> Result<Vec<IndexInfo>>;

    fn get_collection_reader<'a>(&'a self, db: &str, coll: &str, plan: Option<QueryPlan>) -> Result<Box<StorageCollectionReader + 'a>>;
}

pub trait StorageCollectionWriter {
    fn insert(&mut self, v: &BsonValue) -> Result<()>;
    fn update(&mut self, v: &BsonValue) -> Result<()>;
    fn delete(&mut self, v: &BsonValue) -> Result<bool>;
}

// TODO should implement Drop = rollback
// TODO do we need to declare that StorageWriter must implement Drop ?
// TODO or is it enough that the actual implementation of this trait impl Drop?

pub trait StorageWriter : StorageReader {
    fn create_collection(&self, db: &str, coll: &str, options: BsonValue) -> Result<bool>;
    fn rename_collection(&self, old_name: &str, new_name: &str, drop_target: bool) -> Result<bool>;
    fn clear_collection(&self, db: &str, coll: &str) -> Result<bool>;
    fn drop_collection(&self, db: &str, coll: &str) -> Result<bool>;

    fn create_indexes(&self, Vec<IndexInfo>) -> Result<Vec<bool>>;
    fn drop_index(&self, db: &str, coll: &str, name: &str) -> Result<bool>;

    fn drop_database(&self, db: &str) -> Result<bool>;

    fn get_collection_writer<'a>(&'a self, db: &str, coll: &str) -> Result<Box<StorageCollectionWriter + 'a>>;

    fn commit(self: Box<Self>) -> Result<()>;
    fn rollback(self: Box<Self>) -> Result<()>;

}

pub trait StorageConnection {
    fn begin_write<'a>(&'a self) -> Result<Box<StorageWriter + 'a>>;
    fn begin_read<'a>(&'a self) -> Result<Box<StorageReader + 'a>>;
}

pub struct Connection {
    conn: Box<StorageConnection>,
}

impl Connection {
    pub fn new(conn: Box<StorageConnection>) -> Connection {
        Connection {
            conn: conn,
        }
    }

    pub fn insert(&self, db: &str, coll: &str, docs: &Vec<BsonValue>) -> Result<Vec<Result<()>>> {
        // TODO make sure every doc has an _id
        let mut results = Vec::new();
        {
            let writer = try!(self.conn.begin_write());
            {
                let mut collwriter = try!(writer.get_collection_writer(db, coll));
                for doc in docs {
                    let r = collwriter.insert(doc);
                    results.push(r);
                }
            }
            try!(writer.commit());
        }
        Ok(results)
    }

    // TODO consider constraining to just one db.  if so, pass that down to storage?
    pub fn list_collections(&self) -> Result<Vec<(String, String, BsonValue)>> {
        let reader = try!(self.conn.begin_read());
        let v = try!(reader.list_collections());
        Ok(v)
    }

    // TODO consider constraining to just one db or coll.  if so, pass that down to storage?
    pub fn list_indexes(&self) -> Result<Vec<IndexInfo>> {
        let reader = try!(self.conn.begin_read());
        let v = try!(reader.list_indexes());
        Ok(v)
    }

    fn try_find_index_by_name_or_spec(indexes: Vec<IndexInfo>, desc: &BsonValue) -> Option<IndexInfo> {
        let mut a =
            match desc {
                &BsonValue::BString(ref s) => {
                    indexes.into_iter().filter(|ndx| ndx.name.as_str() == s.as_str()).collect::<Vec<_>>()
                },
                &BsonValue::BDocument(_) => {
                    indexes.into_iter().filter(|ndx| ndx.spec == *desc).collect::<Vec<_>>()
                },
                _ => panic!("must be name or index spec doc"),
            };
        if a.len() > 1 {
            unreachable!();
        } else {
            a.pop()
        }
    }

    pub fn delete_indexes(&self, db: &str, coll: &str, index: &BsonValue) -> Result<(usize, usize)> {
        let writer = try!(self.conn.begin_write());
        let indexes = try!(writer.list_indexes()).into_iter().filter(
            |ndx| ndx.db == db && ndx.coll == coll
            ).collect::<Vec<_>>();
        let count_before = indexes.len();
        let indexes = 
            if index.is_string() && try!(index.getString()) == "*" {
                indexes.into_iter().filter(
                    |ndx| ndx.name != "_id_"
                ).collect::<Vec<_>>()
            } else {
                // TODO we're supposed to disallow delete of _id_, right?
                match Self::try_find_index_by_name_or_spec(indexes, index) {
                    Some(ndx) => vec![ndx],
                    None => vec![],
                }
            };
        try!(writer.commit());
        unimplemented!();
    }

    pub fn create_indexes(&self, indexes: Vec<IndexInfo>) -> Result<Vec<bool>> {
        let writer = try!(self.conn.begin_write());
        let results = try!(writer.create_indexes(indexes));
        try!(writer.commit());
        Ok(results)
    }

    pub fn drop_collection(&self, db: &str, coll: &str) -> Result<bool> {
        let deleted = {
            let writer = try!(self.conn.begin_write());
            let deleted = try!(writer.drop_collection(db, coll));
            try!(writer.commit());
            deleted
        };
        Ok(deleted)
    }

    pub fn drop_database(&self, db: &str) -> Result<bool> {
        let deleted = {
            let writer = try!(self.conn.begin_write());
            let deleted = try!(writer.drop_database(db));
            try!(writer.commit());
            deleted
        };
        Ok(deleted)
    }
}

