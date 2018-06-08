console.log("Contour Worker initiated");

self.addEventListener('message', function(e) {
    importScripts("marchingsquares-isobands.min.js");
    importScripts("marchingsquares-isocontours.min.js");
    
    console.log("[CRWORKER]: level = " + e.data.level);

    var band = MarchingSquaresJS.isoBands(e.data.data, e.data.lowerBand, e.data.upperBand - e.data.lowerBand);
    
    self.postMessage(band);

    self.close();
}, false);
