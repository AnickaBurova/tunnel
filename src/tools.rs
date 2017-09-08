/**
 * File: src/tools.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 07.09.2017
 * Last Modified Date: 07.09.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */

macro_rules! io_res {
    ($e: expr) => {
        $e.map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    };
    ($e: expr, $k: ident) => {
        $e.map_err(|e| io::Error::new(io::ErrorKind::$k, e))
    };
    (opt => $e: expr, $msg: expr) => {
        $e.ok_or(io::Error::new(io::ErrorKind::Other, $msg))
    };
    (opt => $e: expr, $k: ident, $msg: expr) => {
        $e.ok_or(io::Error::new(io::ErrorKind::$k, $msg))
    };
}

