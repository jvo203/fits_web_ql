use rusqlite;
use serde_json;

#[derive(Debug)]
pub struct Molecule {
    species: String,
    name: String,
    frequency: f64,
    qn: String,
    cdms_intensity: f64,
    lovas_intensity: f64,
    e_l: f64,
    linelist: String,
}

impl Molecule {
    pub fn from_sqlite_row(row: &rusqlite::Row) -> Molecule {
        Molecule {
            species: match row.get_checked(0) {
                Ok(x) => x,
                Err(_) => String::from(""),
            },

            name: match row.get_checked(1) {
                Ok(x) => x,
                Err(_) => String::from(""),
            },

            frequency: match row.get_checked(2) {
                Ok(x) => x,
                Err(_) => 0.0,
            },

            qn: match row.get_checked(3) {
                Ok(x) => x,
                Err(_) => String::from(""),
            },

            cdms_intensity: match row.get_checked(4) {
                Ok(x) => x,
                Err(_) => 0.0,
            },

            lovas_intensity: match row.get_checked(5) {
                Ok(x) => x,
                Err(_) => 0.0,
            },

            e_l: match row.get_checked(6) {
                Ok(x) => x,
                Err(_) => 0.0,
            },

            linelist: match row.get_checked(7) {
                Ok(x) => x,
                Err(_) => String::from(""),
            },
        }
    }

    pub fn to_json(&self) -> serde_json::value::Value {
        json!({            
            "species" : self.species,
            "name" : self.name,
            "frequency" : self.frequency,
            "quantum" : self.qn,
            "cdms" : self.cdms_intensity,
            "lovas" : self.lovas_intensity,
            "E_L" : self.e_l,
            "list" : self.linelist
        })
    }
}
