function pad(num, size) {
    var s = num + "";
    while (s.length < size) s = "0" + s;
    return s;
};

function randomise_alma() {
    var timestamp = new Date();
    var Id = 500 + timestamp.getUTCMilliseconds();

    var datasetId = "ALMA0101" + pad(Id.toString(), 4);
    document.getElementById("datasetid").value = datasetId;

    view_alma();
}

function view_akari() {
    var dataId = document.getElementById("akari_dataid").value.trim();
    var db = document.getElementById("akari_db").value.trim();
    var table = document.getElementById("akari_table").value.trim();
    var colourmap = document.getElementById("akari_colourmap").value.trim();

    if (dataId != "") {
        var url = null;

        url = "/fitswebql/FITSWebQL.html?" + "db=" + encodeURIComponent(db) + "&table=" + encodeURIComponent(table) + "&datasetId=" + encodeURIComponent(dataId) + "&colourmap=" + encodeURIComponent(colourmap);

        window.location.href = url;
    }
    else
        alert("no datasetId found !");
}

function view_alma() {
    var datasetId = document.getElementById("datasetid").value.trim();
    var db = document.getElementById("alma_db").value.trim();
    var table = document.getElementById("alma_table").value.trim();

    if (datasetId != "") {
        var url = null;

        url = "/fitswebql/FITSWebQL.html?" + "db=" + encodeURIComponent(db) + "&table=" + encodeURIComponent(table) + "&datasetId=" + encodeURIComponent(datasetId);

        window.location.href = url;
    }
    else
        alert("no datasetId found !");
}

function view_url() {
    var fits_url = document.getElementById("url").value.trim();

    if (fits_url != "") {
        var url = null;

        url = "/fitswebql/FITSWebQL.html?" + "url=" + encodeURIComponent(fits_url);

        window.location.href = url;
    }
    else
        alert("no FITS URL found !");
}

function view_hsc() {
    var dataId = document.getElementById("hsc_dataid").value.trim();

    if (dataId == "") {
        alert("no datasetId found !");
        return;
    }

    var va_count = 0;

    var elems = document.getElementsByClassName("hsc_filter");

    for (let i = 0; i < elems.length; i++) {
        if (elems[i].checked)
            va_count++;
    }

    if (va_count == 0) {
        alert("no filter selected !");
        return;
    }

    console.log("va_count = ", va_count);

    var db = document.getElementById("hsc_db").value.trim();
    var table = document.getElementById("hsc_table").value.trim();
    var composite = false;
    var optical = true;

    var url = "/fitswebql/FITSWebQL.html?db=" + encodeURIComponent(db) + "&table=" + encodeURIComponent(table);

    if (va_count == 1) {
        for (let i = 0; i < elems.length; i++)
            if (elems[i].checked)
                url += "&datasetId=" + encodeURIComponent(dataId + "_" + elems[i].getAttribute("id").trim());
    }

    if (va_count > 1) {
        va_count = 0;

        for (let i = 0; i < elems.length; i++)
            if (elems[i].checked)
                url += "&datasetId" + (++va_count) + "=" + encodeURIComponent(dataId + "_" + elems[i].getAttribute("id").trim());

        if (va_count <= 3) {
            composite = document.getElementById("hsc_composite").checked;
        }
    }

    var flux = document.getElementById("hsc_flux").value.trim();
    url += "&flux=" + encodeURIComponent(flux);

    var colourmap = document.getElementById("hsc_colourmap").value.trim();
    url += "&colourmap=" + encodeURIComponent(colourmap);

    if (composite && optical) {
        url += "&view=composite,optical";
    } else {
        if (composite)
            url += "&view=composite";

        if (optical)
            url += "&view=optical";
    }

    window.location.href = url;

}

function view_nro45m() {
    var va_count = 0;

    var elems = document.getElementsByClassName("datasetid");

    for (let i = 0; i < elems.length; i++) {
        if (elems[i].value.trim() != "")
            va_count++;
    }

    if (va_count == 0) {
        alert("no datasetId found !");
        return;
    }

    console.log("va_count = ", va_count);

    var db = document.getElementById("nro_db").value.trim();
    var table = document.getElementById("nro_table").value.trim();

    var url = "/fitswebql/FITSWebQL.html?db=" + encodeURIComponent(db) + "&table=" + encodeURIComponent(table);

    if (va_count == 1) {
        for (let i = 0; i < elems.length; i++)
            if (elems[i].value.trim() != "")
                url += "&datasetId=" + encodeURIComponent(elems[i].value.trim());
    }

    if (va_count > 1) {
        va_count = 0;

        for (let i = 0; i < elems.length; i++)
            if (elems[i].value.trim() != "")
                url += "&datasetId" + (++va_count) + "=" + encodeURIComponent(elems[i].value.trim());

        if (va_count <= 3) {
            var composite = document.getElementById("composite").checked;

            if (composite)
                url += "&view=composite";
        }
    }

    var flux = document.getElementById("nro_flux").value.trim();

    //enforce a tone mapping
    //if(table == "fugin.fugin_meta")    
    url += "&flux=" + encodeURIComponent(flux);

    //console.log(url) ;
    window.location.href = url;
}
