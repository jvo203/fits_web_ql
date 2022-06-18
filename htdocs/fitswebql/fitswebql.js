function get_js_version() {
	return "JS2022-06-18.1";
}

const wasm_supported = (() => {
	try {
		console.log("checking for WebAssembly support");
		if (typeof WebAssembly === "object"
			&& typeof WebAssembly.instantiate === "function") {
			const module = new WebAssembly.Module(Uint8Array.of(0x0, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00));
			if (module instanceof WebAssembly.Module)
				return new WebAssembly.Instance(module) instanceof WebAssembly.Instance;
		}
	} catch (e) {
	}
	return false;
})();

console.log(wasm_supported ? "WebAssembly is supported" : "WebAssembly is not supported");

Array.prototype.rotate = function (n) {
	return this.slice(n, this.length).concat(this.slice(0, n));
}

function clamp(value, min, max) {
	return Math.min(Math.max(min, value), max)
}

function round(value, precision, mode) {
	//  discuss at: http://locutus.io/php/round/
	// original by: Philip Peterson
	//  revised by: Onno Marsman (https://twitter.com/onnomarsman)
	//  revised by: T.Wild
	//  revised by: Rafał Kukawski (http://blog.kukawski.pl)
	//    input by: Greenseed
	//    input by: meo
	//    input by: William
	//    input by: Josep Sanz (http://www.ws3.es/)
	// bugfixed by: Brett Zamir (http://brett-zamir.me)
	//      note 1: Great work. Ideas for improvement:
	//      note 1: - code more compliant with developer guidelines
	//      note 1: - for implementing PHP constant arguments look at
	//      note 1: the pathinfo() function, it offers the greatest
	//      note 1: flexibility & compatibility possible
	//   example 1: round(1241757, -3)
	//   returns 1: 1242000
	//   example 2: round(3.6)
	//   returns 2: 4
	//   example 3: round(2.835, 2)
	//   returns 3: 2.84
	//   example 4: round(1.1749999999999, 2)
	//   returns 4: 1.17
	//   example 5: round(58551.799999999996, 2)
	//   returns 5: 58551.8

	var m, f, isHalf, sgn // helper variables
	// making sure precision is integer
	precision |= 0
	m = Math.pow(10, precision)
	value *= m
	// sign of the number
	sgn = (value > 0) | -(value < 0)
	isHalf = value % 1 === 0.5 * sgn
	f = Math.floor(value)

	if (isHalf) {
		switch (mode) {
			case 'PHP_ROUND_HALF_DOWN':
				// rounds .5 toward zero
				value = f + (sgn < 0)
				break
			case 'PHP_ROUND_HALF_EVEN':
				// rouds .5 towards the next even integer
				value = f + (f % 2 * sgn)
				break
			case 'PHP_ROUND_HALF_ODD':
				// rounds .5 towards the next odd integer
				value = f + !(f % 2)
				break
			default:
				// rounds .5 away from zero
				value = f + (sgn > 0)
		}
	}

	return (isHalf ? value : Math.round(value)) / m
}

// ----------------------------------------------------------
// If you're not in IE (or IE version is less than 5) then:
// ie === undefined
// If you're in IE (>=5) then you can determine which version:
// ie === 7; // IE7
// Thus, to detect IE:
// if (ie) {}
// And to detect the version:
// ie === 6 // IE6
// ie > 7 // IE8, IE9, IE10 ...
// ie < 9 // Anything less than IE9
// ----------------------------------------------------------
var ie = (function () {
	var undef, rv = -1; // Return value assumes failure.
	var ua = window.navigator.userAgent;
	var msie = ua.indexOf('MSIE ');
	var trident = ua.indexOf('Trident/');

	if (msie > 0) {
		// IE 10 or older => return version number
		rv = parseInt(ua.substring(msie + 5, ua.indexOf('.', msie)), 10);
	} else if (trident > 0) {
		// IE 11 (or newer) => return version number
		var rvNum = ua.indexOf('rv:');
		rv = parseInt(ua.substring(rvNum + 3, ua.indexOf('.', rvNum)), 10);
	}

	return ((rv > -1) ? rv : undef);
}());

var colours = ["red", "green", "lightblue"];
var linedash = [[], [10, 5], [5, 5, 2, 2]];

function get_axes_range(width, height) {
	var xMin = /*0.005*width ;*/0.025 * width;
	var xMax = width - xMin - 1;

	var yMin = 0.05 * height;
	var yMax = height - yMin - 1;

	var range = {
		xMin: Math.round(xMin),
		xMax: Math.round(xMax),
		yMin: Math.round(yMin),
		yMax: Math.round(yMax)
	};

	return range;
}

function get_screen_scale(x) {
	//return Math.floor(0.925*x) ;
	return Math.floor(0.9 * x);
}

function get_image_scale_square(width, height, img_width, img_height) {
	var screen_dimension = get_screen_scale(Math.min(width, height));
	var image_dimension = Math.max(img_width, img_height);

	return screen_dimension / image_dimension;
}

function get_image_scale(width, height, img_width, img_height) {
	if (img_width == img_height)
		return get_image_scale_square(width, height, img_width, img_height);

	if (img_height < img_width) {
		var screen_dimension = 0.9 * height;
		var image_dimension = img_height;

		var scale = screen_dimension / image_dimension;

		var new_image_width = scale * img_width;

		if (new_image_width > 0.8 * width) {
			screen_dimension = 0.8 * width;
			image_dimension = img_width;
			scale = screen_dimension / image_dimension;
		}

		return scale;
	}

	if (img_width < img_height) {
		var screen_dimension = 0.8 * width;
		var image_dimension = img_width;

		var scale = screen_dimension / image_dimension;

		var new_image_height = scale * img_height;

		if (new_image_height > 0.9 * height) {
			screen_dimension = 0.9 * height;
			image_dimension = img_height;
			scale = screen_dimension / image_dimension;
		}

		return scale;
	}
}

function get_spectrum_direction(fitsData) {
	var reverse = false;

	if (has_velocity_info) {
		var has_frequency = false;

		if (va_count <= 1) {
			if (RESTFRQ > 0.0)
				has_frequency = true;
		};

		if (!has_frequency) {
			if (fitsData.CDELT3 > 0.0)
				reverse = false;
			else
				reverse = true;
		}
		else {
			if (fitsData.CDELT3 < 0.0)
				reverse = false;
			else
				reverse = true;
		}
	}
	else {
		//ALMAWebQLv2 behaviour
		if (fitsData.CDELT3 > 0.0)
			reverse = false;
		else
			reverse = true;
	};

	return reverse;
}

function get_freq2vel_bounds(freq_start, freq_end, fitsData) {
	if (fitsData.RESTFRQ <= 0.0 && RESTFRQ <= 0.0)
		return { frame_start: 0, frame_end: fitsData.depth - 1 };

	let c = 299792458;//speed of light [m/s]

	var fRatio, v1, v2, x1, x2;

	fRatio = freq_start / RESTFRQ;
	v1 = (1.0 - fRatio * fRatio) / (1.0 + fRatio * fRatio) * c;

	fRatio = freq_end / RESTFRQ;
	v2 = (1.0 - fRatio * fRatio) / (1.0 + fRatio * fRatio) * c;

	x1 = fitsData.CRPIX3 + (v1 - fitsData.CRVAL3) / fitsData.CDELT3 - 1.0;
	x2 = fitsData.CRPIX3 + (v2 - fitsData.CRVAL3) / fitsData.CDELT3 - 1.0;

	var _frame_start = Math.round(x1);
	var _frame_end = Math.round(x2);

	if (_frame_end < _frame_start) {
		let tmp = _frame_start;
		_frame_start = _frame_end;
		_frame_end = tmp;
	};

	_frame_start = Math.max(_frame_start, 0);
	_frame_start = Math.min(_frame_start, fitsData.depth - 1);

	_frame_end = Math.max(_frame_end, 0);
	_frame_end = Math.min(_frame_end, fitsData.depth - 1);

	return { frame_start: _frame_start, frame_end: _frame_end };
}

function get_velocity_bounds(vel_start, vel_end, fitsData) {
	console.log("get_velocity_bounds(" + data_band_lo + "," + data_band_hi + ")");

	var v1, v2, vel_lo, vel_hi;

	v1 = fitsData.CRVAL3 + fitsData.CDELT3 * (1.0 - fitsData.CRPIX3);
	v2 = fitsData.CRVAL3 + fitsData.CDELT3 * (fitsData.depth - fitsData.CRPIX3);

	vel_lo = Math.min(v1, v2);
	vel_hi = Math.max(v1, v2);

	console.log("vel_lo:", vel_lo, "vel_hi:", vel_hi);

	var _frame_start, _frame_end;

	if (fitsData.CDELT3 > 0.0) {
		_frame_start = Math.round((vel_start - vel_lo) / (vel_hi - vel_lo) * (fitsData.depth - 1));
		_frame_end = Math.round((vel_end - vel_lo) / (vel_hi - vel_lo) * (fitsData.depth - 1));
	}
	else {
		_frame_start = Math.round((vel_hi - vel_start) / (vel_hi - vel_lo) * (fitsData.depth - 1));
		_frame_end = roundf((vel_hi - vel_end) / (vel_hi - vel_lo) * (fitsData.depth - 1));
	};

	if (_frame_end < _frame_start) {
		let tmp = _frame_start;
		_frame_start = _frame_end;
		_frame_end = tmp;
	};

	_frame_start = Math.max(_frame_start, 0);
	_frame_start = Math.min(_frame_start, fitsData.depth - 1);

	_frame_end = Math.max(_frame_end, 0);
	_frame_end = Math.min(_frame_end, fitsData.depth - 1);

	return { frame_start: _frame_start, frame_end: _frame_end };
}

function get_frame_bounds(lo, hi, index) {
	//ref_freq = RESTFRQ
	let fitsData = fitsContainer[index];

	if (fitsData == null)
		return { frame_start: 0, frame_end: 0 };

	if (fitsData.depth <= 1)
		return { frame_start: 0, frame_end: 0 };

	if (has_velocity_info && RESTFRQ > 0.0)
		return get_freq2vel_bounds(lo, hi, fitsData);

	if (has_frequency_info)
		return get_frequency_bounds(lo, hi, fitsData);

	if (has_velocity_info)
		return get_velocity_bounds(lo, hi, fitsData);
}

function largestTriangleThreeBuckets(data, threshold) {

	var floor = Math.floor,
		abs = Math.abs;

	var dataLength = data.length;
	if (threshold >= dataLength || threshold === 0) {
		return data; // Nothing to do
	}

	console.log("applying 'largestTriangleThreeBuckets'");

	var sampled = [],
		sampledIndex = 0;

	// Bucket size. Leave room for start and end data points
	var every = (dataLength - 2) / (threshold - 2);

	var a = 0,  // Initially a is the first point in the triangle
		maxAreaPoint,
		maxArea,
		area,
		nextA;

	sampled[sampledIndex++] = data[a]; // Always add the first point

	for (var i = 0; i < threshold - 2; i++) {

		// Calculate point average for next bucket (containing c)
		var avgX = 0,
			avgY = 0,
			avgRangeStart = floor((i + 1) * every) + 1,
			avgRangeEnd = floor((i + 2) * every) + 1;
		avgRangeEnd = avgRangeEnd < dataLength ? avgRangeEnd : dataLength;

		var avgRangeLength = avgRangeEnd - avgRangeStart;

		for (; avgRangeStart < avgRangeEnd; avgRangeStart++) {
			avgX += avgRangeStart;//data[ avgRangeStart ][ xAccessor ] * 1; // * 1 enforces Number (value may be Date)
			avgY += data[avgRangeStart];
		}
		avgX /= avgRangeLength;
		avgY /= avgRangeLength;

		// Get the range for this bucket
		var rangeOffs = floor((i + 0) * every) + 1,
			rangeTo = floor((i + 1) * every) + 1;

		// Point a
		var pointAX = a,//data[ a ][ xAccessor ] * 1, // enforce Number (value may be Date)
			pointAY = data[a];

		maxArea = area = -1;

		for (; rangeOffs < rangeTo; rangeOffs++) {
			// Calculate triangle area over three buckets
			area = abs((pointAX - avgX) * (data[rangeOffs] - pointAY) -
				(pointAX - rangeOffs) * (avgY - pointAY)
			) * 0.5;
			if (area > maxArea) {
				maxArea = area;
				maxAreaPoint = data[rangeOffs];
				nextA = rangeOffs; // Next a is this b
			}
		}

		sampled[sampledIndex++] = maxAreaPoint; // Pick this point from the bucket
		a = nextA; // This a is the next a (chosen b)
	}

	sampled[sampledIndex++] = data[dataLength - 1]; // Always add last

	return sampled;
}

function getShadowStyle() {
	if (!composite_view) {
		if (theme == 'bright')
			return "black";// was purple
		else
			//return "yellow";//was red
			return "rgba(255,204,0,1.0)"; // light amber
	}
	else {
		//return "yellow";
		return "rgba(255,204,0,1.0)"; // light amber
	};
}

function getStrokeStyle() {
	var style = "rgba(0,0,0,1.0)";

	//style = "rgba(255,204,0,0.9)" ;//yellowish ALMAWebQL v2
	style = "rgba(255,255,255,1.0)";//white
	//style = "rgba(153, 102, 153, 0.9)" ;//violet

	if (theme == 'bright') {
		//style = "rgba(0,0,0,1.0)";//black
		style = "rgba(127,127,127,1.0)";// grey

		if (colourmap == "greyscale")
			style = "rgba(255,204,0,1.0)";//yellowish ALMAWebQL v2	    
	}

	if (theme == 'dark') {
		if (colourmap == "green")
			//style = "rgba(255,127,80,0.9)";//orange
			//style = "rgba(238,130,238,0.9)" ;
			//style = "rgba(204,204,204,0.9)";//grey
			style = "rgba(255,204,0,1.0)";//yellowish ALMAWebQL v2	    
		//style = "rgba(204,204,204,1.0)";//grey

		if (colourmap == "red")
			style = "rgba(0,191,255,1.0)";//deepskyblue

		if (colourmap == "blue")
			style = "rgba(255,215,0,1.0)";//gold

		if (colourmap == "hot")
			style = "rgba(0,191,255,1.0)";//deepskyblue

		//if(document.getElementById('colourmap').value == "rainbow")// || document.getElementById('colourmap').value == "parula" || document.getElementById('colourmap').value == "viridis")
		//	style = "rgba(204,204,204,0.9)" ;
	}

	return style;
}

function plot_spectrum(dataArray) {
	if (mousedown)
		return;

	let len = dataArray.length;
	if (len < 1)
		return;

	let fitsData = fitsContainer[len - 1];
	if (fitsData.depth <= 1 || optical_view)
		return;

	var elem = document.getElementById("SpectrumCanvas");
	if (displaySpectrum) {
		elem.style.display = "block";
		d3.select("#yaxis").attr("opacity", 1);
		d3.select("#ylabel").attr("opacity", 1);
	}
	else {
		elem.style.display = "none";
		d3.select("#yaxis").attr("opacity", 0);
		d3.select("#ylabel").attr("opacity", 0);
	}

	var canvas = document.getElementById("SpectrumCanvas");
	var ctx = canvas.getContext('2d');

	var width = canvas.width;
	var height = canvas.height;

	var dmin = 0;
	var dmax = 0;

	tmp_data_min = Number.MAX_VALUE;
	tmp_data_max = - Number.MAX_VALUE;

	for (let index = 0; index < len; index++) {
		let data = dataArray[index];
		let scale = spectrum_scale[index];

		tmp_data_min = Math.min(tmp_data_min, scale * d3.min(data));
		tmp_data_max = Math.max(tmp_data_max, scale * d3.max(data));
	}

	if (autoscale) {
		dmin = tmp_data_min;
		dmax = tmp_data_max;
	}
	else {
		if ((user_data_min != null) && (user_data_max != null)) {
			dmin = user_data_min;
			dmax = user_data_max;
		}
		else {
			dmin = data_min;
			dmax = data_max;
		}
	};

	if (windowLeft) {
		dmin = data_min;
		dmax = data_max;
	}

	if (dmin == dmax) {
		if (dmin == 0.0 && dmax == 0.0) {
			dmin = -1.0;
			dmax = 1.0;
		} else {
			if (dmin > 0.0) {
				dmin *= 0.99;
				dmax *= 1.01;
			};

			if (dmax < 0.0) {
				dmax *= 0.99;
				dmin *= 1.01;
			}
		}
	}

	var range = get_axes_range(width, height);

	var dx = range.xMax - range.xMin;
	var dy = range.yMax - range.yMin;

	var interval = dmax - dmin;
	dmin -= get_spectrum_margin() * interval;
	dmax += get_spectrum_margin() * interval;

	ctx.clearRect(0, 0, width, height);

	//iterate through all spectral lines
	for (let index = 0; index < len; index++) {
		let data = dataArray[index];
		let scale = spectrum_scale[index];

		data = largestTriangleThreeBuckets(data, dx / 2);

		var incrx = dx / (data.length - 1);
		var offset = range.xMin;

		//get display direction
		var reverse = get_spectrum_direction(fitsData);

		var y = 0;

		if (reverse)
			y = (scale * data[data.length - 1] - dmin) / (dmax - dmin) * dy;
		else
			y = (scale * data[0] - dmin) / (dmax - dmin) * dy;

		ctx.save();
		ctx.beginPath();

		ctx.moveTo(offset, range.yMax - y);
		offset += incrx;

		for (var x = 1 | 0; x < data.length; x = (x + 1) | 0) {
			if (reverse)
				y = (scale * data[data.length - 1 - x] - dmin) / (dmax - dmin) * dy;
			else
				y = (scale * data[x] - dmin) / (dmax - dmin) * dy;

			ctx.lineTo(offset, range.yMax - y);
			offset += incrx;
		};

		ctx.shadowColor = getShadowStyle();
		ctx.shadowBlur = 5;//20
		//ctx.shadowOffsetX = 10; 
		//ctx.shadowOffsetY = 10;

		ctx.strokeStyle = getStrokeStyle();

		if (len > 1) {
			//ctx.strokeStyle = colours[index % colours.length] ;
			ctx.setLineDash(linedash[index % linedash.length]);
		}

		ctx.lineWidth = 1; // was 0
		ctx.strokeWidth = emStrokeWidth;

		ctx.stroke();
		ctx.closePath();
		ctx.restore();
	}

	//plot a zero line
	if (va_count == 1)
		if (dmin <= 0 && dmax >= 0) {
			ctx.save();
			ctx.beginPath();

			ctx.shadowColor = getShadowStyle();
			ctx.shadowBlur = 20;
			//ctx.shadowOffsetX = 10; 
			//ctx.shadowOffsetY = 10;
			ctx.strokeStyle = getStrokeStyle();

			//ctx.setLineDash([5, 3]);
			ctx.setLineDash([10, 10]);
			ctx.lineWidth = 1;
			ctx.strokeWidth = emStrokeWidth;

			y = (0 - dmin) / (dmax - dmin) * dy;
			ctx.moveTo(range.xMin, range.yMax - y + emStrokeWidth / 2);
			ctx.lineTo(range.xMax, range.yMax - y + emStrokeWidth / 2);

			ctx.stroke();
			ctx.closePath();
			ctx.restore();
		}
}

function replot_y_axis() {
	if (!displaySpectrum || optical_view)
		return;

	var svg = d3.select("#BackSVG");
	var width = parseFloat(svg.attr("width"));
	var height = parseFloat(svg.attr("height"));

	var dmin = 0.0;
	var dmax = 0.0;

	if (autoscale) {
		dmin = tmp_data_min;
		dmax = tmp_data_max;
	}
	else {
		if ((user_data_min != null) && (user_data_max != null)) {
			dmin = user_data_min;
			dmax = user_data_max;
		}
		else {
			dmin = data_min;
			dmax = data_max;
		}
	};

	if (windowLeft) {
		dmin = data_min;
		dmax = data_max;
	}

	if (dmin == dmax) {
		if (dmin == 0.0 && dmax == 0.0) {
			dmin = -1.0;
			dmax = 1.0;
		} else {
			if (dmin > 0.0) {
				dmin *= 0.99;
				dmax *= 1.01;
			};

			if (dmax < 0.0) {
				dmax *= 0.99;
				dmin *= 1.01;
			}
		}
	}

	var interval = dmax - dmin;

	var range = get_axes_range(width, height);

	var yR = d3.scaleLinear()
		.range([range.yMax, range.yMin])
		.domain([dmin - get_spectrum_margin() * interval, dmax + get_spectrum_margin() * interval]);

	var yAxis = d3.axisRight(yR)
		.tickSizeOuter([3])
		//.tickFormat(function(d) { return d.toPrecision(3) ; }) ;
		.tickFormat(function (d) {
			var number;

			if (Math.abs(d) <= 0.001 || Math.abs(d) >= 1000)
				number = d.toExponential();
			else
				number = d;

			if (Math.abs(d) == 0)
				number = d;

			return number;
		});

	d3.select("#yaxis").remove();
	svg = d3.select("#axes");

	// Add the Y Axis
	svg.append("g")
		.attr("class", "axis")
		.attr("id", "yaxis")
		.style("fill", "#996699")
		.style("stroke", "#996699")
		//.style("stroke-width", emStrokeWidth)
		.attr("transform", "translate(" + (0.75 * range.xMin - 1) + ",0)")
		.call(yAxis);

	//y-axis label
	var yLabel = "Integrated";

	if (intensity_mode == "mean")
		yLabel = "Mean";

	let fitsData = fitsContainer[va_count - 1];

	var bunit = '';
	if (fitsData.BUNIT != '') {
		bunit = fitsData.BUNIT.trim();

		if (intensity_mode == "integrated" && has_velocity_info)
			bunit += '•km/s';

		bunit = "[" + bunit + "]";
	}

	d3.select("#ylabel").text(yLabel + ' ' + fitsData.BTYPE.trim() + " " + bunit);
}

function process_image(width, height, w, h, bytes, stride, alpha, index) {
	//let image_bounding_dims = {x1: 0, y1: 0, width: w, height: h};
	let image_bounding_dims = true_image_dimensions(alpha, width, height);
	var pixel_range = image_pixel_range(bytes, w, h, stride);
	console.log("min pixel:", pixel_range.min_pixel, "max pixel:", pixel_range.max_pixel);

	let imageCanvas = document.createElement('canvas');
	imageCanvas.style.visibility = "hidden";
	var context = imageCanvas.getContext('2d');

	imageCanvas.width = width;
	imageCanvas.height = height;
	console.log(imageCanvas.width, imageCanvas.height);

	var imageFrame;
	let imageData = context.createImageData(width, height);

	if (width >= height)
		imageFrame = { bytes: new Uint8ClampedArray(bytes), w: w, h: h, stride: stride };
	else {
		//re-arrange the bytes array
		var buffer = new Uint8Array(w * h);

		let dst_offset = 0;

		for (var j = 0; j < h; j++) {
			let offset = j * stride;

			for (var i = 0; i < w; i++)
				buffer[dst_offset++] = bytes[offset++];
		}

		imageFrame = { bytes: buffer, w: width, h: height, stride: width };
	}

	apply_colourmap(imageData, colourmap, bytes, w, h, stride, alpha);
	context.putImageData(imageData, 0, 0);

	imageContainer[index - 1] = { imageCanvas: imageCanvas, imageFrame: imageFrame, imageData: imageData, alpha: alpha, newImageData: null, image_bounding_dims: image_bounding_dims, pixel_range: pixel_range };

	//next display the image
	if (va_count == 1) {
		//place the image onto the main canvas
		var c = document.getElementById('HTMLCanvas');
		var width = c.width;
		var height = c.height;
		var ctx = c.getContext("2d");

		ctx.mozImageSmoothingEnabled = false;
		ctx.webkitImageSmoothingEnabled = false;
		ctx.msImageSmoothingEnabled = false;
		ctx.imageSmoothingEnabled = false;

		var scale = get_image_scale(width, height, image_bounding_dims.width, image_bounding_dims.height);

		var img_width = scale * image_bounding_dims.width;
		var img_height = scale * image_bounding_dims.height;

		ctx.drawImage(imageCanvas, image_bounding_dims.x1, image_bounding_dims.y1, image_bounding_dims.width, image_bounding_dims.height, (width - img_width) / 2, (height - img_height) / 2, img_width, img_height);

		if (navigation == "dynamic")
			setup_image_selection();

		if (navigation == "static") {
			setup_image_selection_index(1, (width - img_width) / 2, (height - img_height) / 2, img_width, img_height);

			//trigger a tileTimeout
			if (zoom_dims != null)
				if (zoom_dims.view != null)
					tileTimeout(true);
		}

		has_image = true;

		if (navigation == "dynamic")
			setup_viewports();

		try {
			display_scale_info();
		}
		catch (err) {
		};

		display_legend();

		try {
			display_cd_gridlines();
		}
		catch (err) {
			display_gridlines();
		};

		display_beam();

		setup_3d_view();

		hide_hourglass();
	}
	else {
		if (!composite_view) {
			if (zoom_dims != null)
				if (zoom_dims.view != null)
					image_bounding_dims = zoom_dims.view;

			//place the image onto the main canvas
			var c = document.getElementById('HTMLCanvas' + index);
			var width = c.width;
			var height = c.height;
			var ctx = c.getContext("2d");

			ctx.mozImageSmoothingEnabled = false;
			ctx.webkitImageSmoothingEnabled = false;
			ctx.msImageSmoothingEnabled = false;
			ctx.imageSmoothingEnabled = false;
			//ctx.globalAlpha=0.9;

			var scale = get_image_scale(width, height, image_bounding_dims.width, image_bounding_dims.height);

			if (va_count == 2)
				scale = 0.8 * scale;
			else if (va_count == 4)
				scale = 0.6 * scale;
			else if (va_count == 5)
				scale = 0.5 * scale;
			else if (va_count == 6)
				scale = 0.45 * scale;
			else if (va_count == 7)
				scale = 0.45 * scale;
			else
				scale = 2 * scale / va_count;

			var img_width = scale * image_bounding_dims.width;
			var img_height = scale * image_bounding_dims.height;

			let image_position = get_image_position(index, width, height);
			let posx = image_position.posx;
			let posy = image_position.posy;

			ctx.drawImage(imageCanvas, image_bounding_dims.x1, image_bounding_dims.y1, image_bounding_dims.width, image_bounding_dims.height, Math.round(posx - img_width / 2), Math.round(posy - img_height / 2), Math.round(img_width), Math.round(img_height));

			//add a bounding box			
			if (theme == 'bright')
				ctx.strokeStyle = "white";
			else
				ctx.strokeStyle = "black";

			ctx.lineWidth = 2;

			ctx.rect(Math.round(posx - img_width / 2), Math.round(posy - img_height / 2), Math.round(img_width), Math.round(img_height));
			ctx.stroke();
			//end of a bounding box

			add_line_label(index);

			setup_image_selection_index(index, posx - img_width / 2, posy - img_height / 2, img_width, img_height);

			//trigger a tileTimeout
			if (zoom_dims != null)
				if (zoom_dims.view != null)
					tileTimeout(true);
		}
		else
		//add a channel to the RGB composite image
		{
			if (compositeCanvas == null) {
				compositeCanvas = document.createElement('canvas');
				compositeCanvas.style.visibility = "hidden";
				//compositeCanvas = document.getElementById('CompositeCanvas');

				compositeCanvas.width = imageCanvas.width;
				compositeCanvas.height = imageCanvas.height;
			}

			if (compositeImageData == null) {
				var ctx = compositeCanvas.getContext('2d');
				compositeImageData = ctx.createImageData(compositeCanvas.width, compositeCanvas.height);
			}

			add_composite_channel(bytes, w, h, stride, alpha, compositeImageData, index - 1);
		}
	}

	image_count++;

	if (image_count == va_count) {
		//display the composite image
		if (composite_view) {
			if (compositeCanvas != null && compositeImageData != null) {
				var tmp = compositeCanvas.getContext('2d');
				tmp.putImageData(compositeImageData, 0, 0);

				//place the image onto the main canvas
				var c = document.getElementById('HTMLCanvas');
				var width = c.width;
				var height = c.height;
				var ctx = c.getContext("2d");

				ctx.mozImageSmoothingEnabled = false;
				ctx.webkitImageSmoothingEnabled = false;
				ctx.msImageSmoothingEnabled = false;
				ctx.imageSmoothingEnabled = false;

				var scale = get_image_scale(width, height, image_bounding_dims.width, image_bounding_dims.height);

				var img_width = scale * image_bounding_dims.width;
				var img_height = scale * image_bounding_dims.height;

				ctx.drawImage(compositeCanvas, image_bounding_dims.x1, image_bounding_dims.y1, image_bounding_dims.width, image_bounding_dims.height, (width - img_width) / 2, (height - img_height) / 2, img_width, img_height);

				//hide multiple images
				for (let index = 0; index < va_count; index++) {
					try {

						d3.select("#image_rectangle" + (index + 1)).remove();
					}
					catch (e) { };

					document.getElementById("HTMLCanvas" + (index + 1)).style.display = "none";
				}

				setup_image_selection();

				setup_viewports();

				console.log(imageContainer);

				try {
					display_scale_info();
				}
				catch (err) {
				};

				display_rgb_legend();

				setup_3d_view();
			}
		}

		has_image = true;

		hide_hourglass();

		try {
			display_cd_gridlines();
		}
		catch (err) {
			display_gridlines();
		};

		display_beam();
	}

	try {
		var element = document.getElementById('BackHTMLCanvas');
		element.parentNode.removeChild(element);
	}
	catch (e) { }
}


function process_video(index) {
	if (!streaming || videoFrame[index - 1] == null || videoFrame[index - 1].img == null || videoFrame[index - 1].ptr == null || videoFrame[index - 1].alpha == null)
		return;

	//let image_bounding_dims = imageContainer[index-1].image_bounding_dims;
	//{x1: 0, y1: 0, width: w, height: h};

	let imageCanvas = document.createElement('canvas');
	imageCanvas.style.visibility = "hidden";
	var context = imageCanvas.getContext('2d');

	let imageData = videoFrame[index - 1].img;
	let image_bounding_dims = videoFrame[index - 1].image_bounding_dims;

	imageCanvas.width = imageData.width;
	imageCanvas.height = imageData.height;
	console.log(imageCanvas.width, imageCanvas.height);

	context.putImageData(imageData, 0, 0);

	if (va_count > 1 && !composite_view) {
		//place imageCanvas onto a corresponding image_rectangle (should really be using video_rectangle here...)

		var c = document.getElementById('HTMLCanvas' + index);
		var width = c.width;
		var height = c.height;
		var ctx = c.getContext("2d");

		ctx.mozImageSmoothingEnabled = false;
		ctx.webkitImageSmoothingEnabled = false;
		ctx.msImageSmoothingEnabled = false;
		ctx.imageSmoothingEnabled = false;

		let img_width = 0, img_height = 0;
		try {
			let elem = d3.select("#image_rectangle" + index);
			img_width = elem.attr("width");
			img_height = elem.attr("height");
		} catch (err) {
			return;
		}

		let image_position = get_image_position(index, width, height);
		let posx = image_position.posx;
		let posy = image_position.posy;

		ctx.drawImage(imageCanvas, image_bounding_dims.x1, image_bounding_dims.y1, image_bounding_dims.width, image_bounding_dims.height, Math.round(posx - img_width / 2), Math.round(posy - img_height / 2), Math.round(img_width), Math.round(img_height));

		//add a bounding box			
		if (theme == 'bright')
			ctx.strokeStyle = "white";
		else
			ctx.strokeStyle = "black";

		ctx.lineWidth = 2;

		ctx.rect(Math.round(posx - img_width / 2), Math.round(posy - img_height / 2), Math.round(img_width), Math.round(img_height));
		ctx.stroke();
		//end of a bounding box

		return;
	}

	//next display the video frame
	//place the image onto the main canvas
	var c = document.getElementById('VideoCanvas');
	var width = c.width;
	var height = c.height;
	var ctx = c.getContext("2d");

	ctx.mozImageSmoothingEnabled = false;
	ctx.webkitImageSmoothingEnabled = false;
	ctx.msImageSmoothingEnabled = false;
	ctx.imageSmoothingEnabled = false;

	var scale = get_image_scale(width, height, image_bounding_dims.width, image_bounding_dims.height);

	var img_width = scale * image_bounding_dims.width;
	var img_height = scale * image_bounding_dims.height;

	ctx.drawImage(imageCanvas, image_bounding_dims.x1, image_bounding_dims.y1, image_bounding_dims.width, image_bounding_dims.height, (width - img_width) / 2, (height - img_height) / 2, img_width, img_height);

	if (viewport_zoom_settings != null) {
		let px = emStrokeWidth;
		let py = emStrokeWidth;

		//and a zoomed viewport
		if (zoom_shape == "square") {
			ctx.fillStyle = "rgba(0,0,0,0.3)";
			ctx.fillRect(px, py, viewport_zoom_settings.zoomed_size, viewport_zoom_settings.zoomed_size);

			ctx.drawImage(imageCanvas, (viewport_zoom_settings.x - viewport_zoom_settings.clipSize) / videoFrame[index - 1].scaleX, (viewport_zoom_settings.y - viewport_zoom_settings.clipSize) / videoFrame[index - 1].scaleY, (2 * viewport_zoom_settings.clipSize + 1) / videoFrame[index - 1].scaleX, (2 * viewport_zoom_settings.clipSize + 1) / videoFrame[index - 1].scaleY, px, py, viewport_zoom_settings.zoomed_size, viewport_zoom_settings.zoomed_size);
		}

		if (zoom_shape == "circle") {
			ctx.save();
			ctx.beginPath();
			ctx.arc(px + viewport_zoom_settings.zoomed_size / 2, py + viewport_zoom_settings.zoomed_size / 2, viewport_zoom_settings.zoomed_size / 2, 0, 2 * Math.PI, true);

			ctx.fillStyle = "rgba(0,0,0,0.3)";
			ctx.fill();

			ctx.closePath();
			ctx.clip();
			ctx.drawImage(imageCanvas, (viewport_zoom_settings.x - viewport_zoom_settings.clipSize) / videoFrame[index - 1].scaleX, (viewport_zoom_settings.y - viewport_zoom_settings.clipSize) / videoFrame[index - 1].scaleY, (2 * viewport_zoom_settings.clipSize + 1) / videoFrame[index - 1].scaleX, (2 * viewport_zoom_settings.clipSize + 1) / videoFrame[index - 1].scaleY, px, py, viewport_zoom_settings.zoomed_size, viewport_zoom_settings.zoomed_size);
			ctx.restore();
		}
	}
}

function process_viewport_canvas(viewportCanvas, index) {
	if (streaming)
		return;

	console.log("process_viewport_canvas(" + index + ")");

	document.getElementById('welcome').style.display = "none";

	viewport_count++;

	if ((va_count > 1 && !composite_view) || navigation == "static") {
		console.log("refresh_viewport for index:", index);

		if (viewport_count == va_count) {
			//hide_hourglass();
			end_blink();
		}

		if ((recv_seq_id < sent_seq_id) || dragging || moving)
			return;

		//place the viewport onto the image tile
		var id;
		if (va_count == 1)
			id = 'HTMLCanvas';
		else
			id = 'HTMLCanvas' + index;
		var c = document.getElementById(id);
		var width = c.width;
		var height = c.height;
		var ctx = c.getContext("2d");

		ctx.mozImageSmoothingEnabled = false;
		ctx.webkitImageSmoothingEnabled = false;
		ctx.msImageSmoothingEnabled = false;
		ctx.imageSmoothingEnabled = false;
		//ctx.globalAlpha=0.9;

		let img_width = 0, img_height = 0;
		try {
			var id;
			if (va_count == 1)
				id = "#image_rectangle";
			else
				id = "#image_rectangle" + index;
			let elem = d3.select(id);
			img_width = elem.attr("width");
			img_height = elem.attr("height");
		} catch (err) {
			return;
		}

		let image_position = get_image_position(index, width, height);
		let posx = image_position.posx;
		let posy = image_position.posy;

		ctx.drawImage(viewportCanvas, 0, 0, viewportCanvas.width, viewportCanvas.height, Math.round(posx - img_width / 2), Math.round(posy - img_height / 2), Math.round(img_width), Math.round(img_height));

		//add a bounding box			
		if (theme == 'bright')
			ctx.strokeStyle = "white";
		else
			ctx.strokeStyle = "black";

		ctx.lineWidth = 2;

		ctx.rect(Math.round(posx - img_width / 2), Math.round(posy - img_height / 2), Math.round(img_width), Math.round(img_height));
		ctx.stroke();
		//end of a bounding box

		return;
	}

	if (viewport_count == va_count) {
		//place the viewport onto the ZOOM Canvas
		var c = document.getElementById("ZOOMCanvas");
		var ctx = c.getContext("2d");

		ctx.mozImageSmoothingEnabled = false;
		ctx.webkitImageSmoothingEnabled = false;
		ctx.msImageSmoothingEnabled = false;
		ctx.imageSmoothingEnabled = false;

		var width = c.width;
		var height = c.height;

		var imageCanvas = imageContainer[index - 1].imageCanvas;
		var scale = get_image_scale(width, height, imageCanvas.width, imageCanvas.height);
		var img_width = scale * imageCanvas.width;
		var img_height = scale * imageCanvas.height;

		var px, py;

		var zoomed_size = get_zoomed_size(width, height, img_width, img_height);

		if (zoom_location == "upper") {
			px = emStrokeWidth;
			py = emStrokeWidth;
		}
		else {
			px = width - 1 - emStrokeWidth - zoomed_size;
			py = height - 1 - emStrokeWidth - zoomed_size;
		}

		zoomed_size = Math.round(zoomed_size);
		px = Math.round(px);
		py = Math.round(py);

		if (/*recv_seq_id == sent_seq_id*/ /*&& !dragging*/ /*&&*/ !moving) {
			ctx.clearRect(px, py, zoomed_size, zoomed_size);

			if (zoom_shape == "square") {
				ctx.fillStyle = "rgba(0,0,0,0.3)";
				ctx.fillRect(px, py, zoomed_size, zoomed_size);

				ctx.drawImage(viewportCanvas, 0, 0, viewportCanvas.width, viewportCanvas.height, px, py, zoomed_size, zoomed_size);
			}

			if (zoom_shape == "circle") {
				ctx.save();
				ctx.beginPath();
				ctx.arc(px + zoomed_size / 2, py + zoomed_size / 2, zoomed_size / 2, 0, 2 * Math.PI, true);

				ctx.fillStyle = "rgba(0,0,0,0.3)";
				ctx.fill();

				ctx.closePath();
				ctx.clip();
				ctx.drawImage(viewportCanvas, 0, 0, viewportCanvas.width, viewportCanvas.height, px, py, zoomed_size, zoomed_size);
				ctx.restore();
			}
		}
		else {
			console.log("cancelled a viewport refresh; recv_seq_id =", recv_seq_id, " sent_seq_id =", sent_seq_id, " moving:", moving);
		}
	}
}

function process_viewport(width, height, w, h, bytes, stride, alpha, index, swap_dims = true) {
	if (streaming)
		return;

	console.log("process_viewport(" + index + ")");

	document.getElementById('welcome').style.display = "none";

	let viewportCanvas = document.createElement('canvas');
	viewportCanvas.style.visibility = "hidden";
	var context = viewportCanvas.getContext('2d');

	viewportCanvas.width = width;
	viewportCanvas.height = height;
	console.log(viewportCanvas.width, viewportCanvas.height);

	viewport_count++;

	var buffer = bytes;
	var _w = w;
	var _h = h;
	var _stride = stride;

	if (width < height && swap_dims) {
		//re-arrange the bytes array
		buffer = new Uint8Array(w * h);

		let dst_offset = 0;

		for (var j = 0; j < h; j++) {
			let offset = j * stride;

			for (var i = 0; i < w; i++)
				buffer[dst_offset++] = bytes[offset++];
		}

		_w = width;
		_h = height;
		_stride = width;
	};

	if (!composite_view) {
		let imageData = context.createImageData(width, height);
		apply_colourmap(imageData, colourmap, buffer, _w, _h, _stride, alpha);
		context.putImageData(imageData, 0, 0);
	}
	else {
		if (compositeViewportCanvas == null) {
			compositeViewportCanvas = document.createElement('canvas');
			compositeViewportCanvas.style.visibility = "hidden";

			compositeViewportCanvas.width = viewportCanvas.width;
			compositeViewportCanvas.height = viewportCanvas.height;
		}

		if (compositeViewportImageData == null) {
			let ctx = compositeViewportCanvas.getContext('2d');
			compositeViewportImageData = ctx.createImageData(compositeViewportCanvas.width, compositeViewportCanvas.height);
		}

		add_composite_channel(buffer, _w, _h, _stride, alpha, compositeViewportImageData, index - 1);

		if (viewport_count == va_count) {
			if (compositeViewportCanvas != null && compositeViewportImageData != null) {
				var tmp = compositeViewportCanvas.getContext('2d');
				tmp.putImageData(compositeViewportImageData, 0, 0);
				viewportCanvas = compositeViewportCanvas;
			}
		}
	}

	if ((va_count > 1 && !composite_view) || navigation == "static") {
		console.log("refresh_viewport for index:", index);

		if (viewport_count == va_count) {
			//hide_hourglass();
			end_blink();
		}

		if ((recv_seq_id < sent_seq_id) || dragging || moving)
			return;

		//place the viewport onto the image tile
		var id;
		if (va_count == 1)
			id = 'HTMLCanvas';
		else
			id = 'HTMLCanvas' + index;
		var c = document.getElementById(id);
		var width = c.width;
		var height = c.height;
		var ctx = c.getContext("2d");

		ctx.mozImageSmoothingEnabled = false;
		ctx.webkitImageSmoothingEnabled = false;
		ctx.msImageSmoothingEnabled = false;
		ctx.imageSmoothingEnabled = false;
		//ctx.globalAlpha=0.9;

		let img_width = 0, img_height = 0;
		try {
			var id;
			if (va_count == 1)
				id = "#image_rectangle";
			else
				id = "#image_rectangle" + index;
			let elem = d3.select(id);
			img_width = elem.attr("width");
			img_height = elem.attr("height");
		} catch (err) {
			return;
		}

		let image_position = get_image_position(index, width, height);
		let posx = image_position.posx;
		let posy = image_position.posy;

		ctx.drawImage(viewportCanvas, 0, 0, viewportCanvas.width, viewportCanvas.height, Math.round(posx - img_width / 2), Math.round(posy - img_height / 2), Math.round(img_width), Math.round(img_height));

		//add a bounding box			
		if (theme == 'bright')
			ctx.strokeStyle = "white";
		else
			ctx.strokeStyle = "black";

		ctx.lineWidth = 2;

		ctx.rect(Math.round(posx - img_width / 2), Math.round(posy - img_height / 2), Math.round(img_width), Math.round(img_height));
		ctx.stroke();
		//end of a bounding box

		return;
	}

	if (viewport_count == va_count) {
		//place the viewport onto the ZOOM Canvas
		var c = document.getElementById("ZOOMCanvas");
		var ctx = c.getContext("2d");

		ctx.mozImageSmoothingEnabled = false;
		ctx.webkitImageSmoothingEnabled = false;
		ctx.msImageSmoothingEnabled = false;
		ctx.imageSmoothingEnabled = false;

		var width = c.width;
		var height = c.height;

		var imageCanvas = imageContainer[index - 1].imageCanvas;
		var scale = get_image_scale(width, height, imageCanvas.width, imageCanvas.height);
		var img_width = scale * imageCanvas.width;
		var img_height = scale * imageCanvas.height;

		var px, py;

		var zoomed_size = get_zoomed_size(width, height, img_width, img_height);

		if (zoom_location == "upper") {
			px = emStrokeWidth;
			py = emStrokeWidth;
		}
		else {
			px = width - 1 - emStrokeWidth - zoomed_size;
			py = height - 1 - emStrokeWidth - zoomed_size;
		}

		zoomed_size = Math.round(zoomed_size);
		px = Math.round(px);
		py = Math.round(py);

		if (/*recv_seq_id == sent_seq_id*/ /*&& !dragging*/ /*&&*/ !moving) {
			ctx.clearRect(px, py, zoomed_size, zoomed_size);

			if (zoom_shape == "square") {
				ctx.fillStyle = "rgba(0,0,0,0.3)";
				ctx.fillRect(px, py, zoomed_size, zoomed_size);

				ctx.drawImage(viewportCanvas, 0, 0, viewportCanvas.width, viewportCanvas.height, px, py, zoomed_size, zoomed_size);
			}

			if (zoom_shape == "circle") {
				ctx.save();
				ctx.beginPath();
				ctx.arc(px + zoomed_size / 2, py + zoomed_size / 2, zoomed_size / 2, 0, 2 * Math.PI, true);

				ctx.fillStyle = "rgba(0,0,0,0.3)";
				ctx.fill();

				ctx.closePath();
				ctx.clip();
				ctx.drawImage(viewportCanvas, 0, 0, viewportCanvas.width, viewportCanvas.height, px, py, zoomed_size, zoomed_size);
				ctx.restore();
			}
		}
		else {
			console.log("cancelled a viewport refresh; recv_seq_id =", recv_seq_id, " sent_seq_id =", sent_seq_id, " moving:", moving);
		}
	}
}

function process_progress_event(data, index) {
	if (data != null) {
		var message = data.message;
		var running = data.running;
		var total = data.total;
		var elapsed = data.elapsed;

		//console.log(data, index);

		if (total > 0) {
			notifications_received[index - 1] = Math.max(running, notifications_received[index - 1]);

			/*if(running > 0)
			PROGRESS_VARIABLE = running/total ;
			else*/
			var PROGRESS_VARIABLE = notifications_received[index - 1] / total;

			if (PROGRESS_VARIABLE != previous_progress[index - 1]) {
				previous_progress[index - 1] = PROGRESS_VARIABLE;

				PROGRESS_INFO = "&nbsp;" + numeral(PROGRESS_VARIABLE).format('0.0%');

				var speed = notifications_received[index - 1] / elapsed;
				var remaining_time = (total - notifications_received[index - 1]) / speed;//[s]

				//console.log("speed:", speed, "remaining:", remaining_time);
				if (remaining_time > 1)
					PROGRESS_INFO += ", " + numeral(remaining_time).format('00:00:00');

				//console.log(PROGRESS_INFO) ;

				d3.select("#progress-bar" + index)
					.attr("aria-valuenow", (100.0 * PROGRESS_VARIABLE))
					.style("width", (100.0 * PROGRESS_VARIABLE) + "%")
					.html(PROGRESS_INFO);
			}
		}
		else {
			notifications_completed++;

			if (notifications_completed == va_count)
				document.getElementById('welcome').style.display = "none";

			/*if (message.indexOf("error") >= 0)
				show_critical_error();*/
		}
	}
}

function getEndianness() {
	var a = new ArrayBuffer(4);
	var b = new Uint8Array(a);
	var c = new Uint32Array(a);
	b[0] = 0xa1;
	b[1] = 0xb2;
	b[2] = 0xc3;
	b[3] = 0xd4;
	if (c[0] === 0xd4c3b2a1) {
		return true;//BlobReader.ENDIANNESS.LITTLE_ENDIAN;
	}
	if (c[0] === 0xa1b2c3d4) {
		return false;//BlobReader.ENDIANNESS.BIG_ENDIAN;
	} else {
		throw new Error('Unrecognized endianness');
	}
}

function send_ping() {
	if (wsConn[va_count - 1] != null) {
		t = performance.now();

		wsConn[va_count - 1].send('[heartbeat] ' + t);
	}
}

function open_websocket_connection(_datasetId, index) {
	if ("WebSocket" in window) {
		//alert("WebSocket is supported by your Browser!");

		// Let us open a web socket
		var loc = window.location, ws_prot, ws_uri;
		var prot = loc.protocol;

		if (prot !== "https:")
			ws_prot = "ws://";
		else
			ws_prot = "wss://";

		// a JVO override (a special exception)
		if (loc.hostname.indexOf("jvo.") != -1 || loc.hostname.indexOf("jvo-dev.") != -1) {
			console.log("JVO detected, switching the WebSocket protocol to 'wss'.");
			ws_prot = "wss://";
		}

		ws_uri = ws_prot + loc.hostname + ':' + loc.port + ROOT_PATH + "websocket/" + encodeURIComponent(_datasetId);

		//d3.select("#welcome").append("p").text("ws_uri: " + ws_uri) ;

		{
			d3.select("#ping")
				.attr("fill", "orange")
				.attr("opacity", 0.8);

			var ALMAWS = new ReconnectingWebSocket(ws_uri, null, { binaryType: 'arraybuffer' });
			ALMAWS.binaryType = 'arraybuffer';

			ALMAWS.addEventListener("open", function (evt) {
				d3.select("#ping")
					.attr("fill", "green")
					.attr("opacity", 0.8);

				ALMAWS.binaryType = 'arraybuffer';

				let log = wasm_supported ? "WebAssembly is supported" : "WebAssembly is not supported";
				ALMAWS.send('[debug] ' + log);

				if (index == va_count) {
					send_ping();
				}
			});

			ALMAWS.addEventListener("error", function (evt) {

				d3.select("#ping")
					.attr("fill", "red")
					.attr("opacity", 0.8);

				d3.select("#latency").text('websocket conn. error');
			});

			ALMAWS.addEventListener("close", function (evt) { });

			ALMAWS.addEventListener("message", function (evt) {
				var t = performance.now();
				var received_msg = evt.data;

				if (evt.data instanceof ArrayBuffer) {
					var dv = new DataView(received_msg);

					latency = performance.now() - dv.getFloat32(0, endianness);
					//console.log("[ws] latency = " + latency.toFixed(1) + " [ms]") ;					
					recv_seq_id = dv.getUint32(4, endianness);
					var type = dv.getUint32(8, endianness);

					//spectrum
					if (type == 0) {
						computed = dv.getFloat32(12, endianness);

						var length = dv.getUint32(16, endianness);

						//var spectrum = new Float32Array(received_msg, 24);//16+8, extra 8 bytes for the length of the vector, added automatically by Rust
						var frame = new Uint8Array(received_msg, 24);

						try {
							var vec = fpzip_decompressor.FPunzip(frame);

							let len = vec.size();

							//console.log("[ws] computed = " + computed.toFixed(1) + " [ms]" + " length: " + length + " spectrum length:" + spectrum.length + " spectrum: " + spectrum);
							if (len > 0) {
								var spectrum = new Float32Array(len);

								for (let i = 0; i < len; i++)
									spectrum[i] = vec.get(i);

								if (!windowLeft) {
									spectrum_stack[index - 1].push({ spectrum: spectrum, id: recv_seq_id });
									console.log("index:", index, "spectrum_stack length:", spectrum_stack[index - 1].length);
								};
							};

							vec.delete();
						} catch (e) { };

						return;
					}

					//viewport
					if (type == 1) {
						var offset = 12;
						var id_length = dv.getUint32(offset, endianness);
						offset += 8;

						var identifier = new Uint8Array(received_msg, offset, id_length);
						identifier = new TextDecoder("utf-8").decode(identifier);
						offset += id_length;

						var width = dv.getUint32(offset, endianness);
						offset += 4;

						var height = dv.getUint32(offset, endianness);
						offset += 4;

						var no_frames = dv.getUint32(offset, endianness);
						offset += 8;

						var frames = new Array(no_frames);

						for (let i = 0; i < no_frames; i++) {
							var image_length = dv.getUint32(offset, endianness);
							offset += 8;

							var frame = new Uint8Array(received_msg, offset, image_length);
							offset += image_length;

							frames[i] = frame;
						}

						var alpha_length = dv.getUint32(offset, endianness);
						offset += 8;

						var alpha = new Uint8Array(received_msg, offset);
						console.log("viewport frame identifier: ", identifier, "width:", width, "height:", height, "no. frames:", no_frames, "compressed alpha length:", alpha.length);

						var Buffer = require('buffer').Buffer;
						var LZ4 = require('lz4');

						var uncompressed = new Buffer(width * height);
						uncompressedSize = LZ4.decodeBlock(new Buffer(alpha), uncompressed);
						alpha = uncompressed.slice(0, uncompressedSize);

						if (identifier == 'VP9') {
							var decoder = new OGVDecoderVideoVP9();

							decoder.init(function () { console.log("init callback done"); });

							for (let i = 0; i < no_frames; i++) {
								decoder.processFrame(frames[i], function () {
									process_viewport(width, height, decoder.frameBuffer.format.displayWidth,
										decoder.frameBuffer.format.displayHeight,
										decoder.frameBuffer.y.bytes,
										decoder.frameBuffer.y.stride,
										alpha,
										index);
								});
							}
						}

						if (identifier == 'HEVC') {
							if (!composite_view) {
								let viewportCanvas = document.createElement('canvas');
								viewportCanvas.style.visibility = "hidden";
								var context = viewportCanvas.getContext('2d');

								viewportCanvas.width = width;
								viewportCanvas.height = height;
								console.log(viewportCanvas.width, viewportCanvas.height);

								//set up canvas_ptr, alpha_ptr and img
								var len = width * height * 4;
								var canvas_ptr = Module._malloc(len);

								var data = new Uint8ClampedArray(Module.HEAPU8.buffer, canvas_ptr, len);
								for (let i = 0; i < len; i++)
									data[i] = 0;
								var img = new ImageData(data, width, height);

								var alpha_ptr = Module._malloc(width * height);
								Module.HEAPU8.set(alpha, alpha_ptr);

								console.log("Module._malloc canvas_ptr=", canvas_ptr, "ImageData=", img, "alpha_ptr=", alpha_ptr);

								try {
									//init the HEVC encoder		
									api.hevc_init(va_count);
								} catch (e) { };

								//hevc decoding
								for (let i = 0; i < no_frames; i++) {
									let frame = frames[i];
									var len = frame.length;
									var ptr = Module._malloc(len);

									Module.HEAPU8.set(frame, ptr);

									try {
										//HEVC
										api.hevc_decode_nal_unit(0, ptr, len, canvas_ptr, img.width, img.height, alpha_ptr, null, colourmap);
									} catch (e) { };

									if (img.data.length == 0) {
										//detect detached data due to WASM memory growth
										console.log("detached WASM buffer detected, refreshing img:ImageData");

										//WASM buffers have changed, need to refresh the ImageData.data buffer
										var len = img.width * img.height * 4;
										var data = new Uint8ClampedArray(Module.HEAPU8.buffer, canvas_ptr, len);
										for (let i = 0; i < len; i++)
											data[i] = 0;
										img = new ImageData(data, img.width, img.height);
									}

									Module._free(ptr);
								}

								context.putImageData(img, 0, 0);
								process_viewport_canvas(viewportCanvas, index);

								try {
									api.hevc_destroy(va_count);
								} catch (e) { };

								Module._free(canvas_ptr);
								Module._free(alpha_ptr);
							} else {
								var bytes_ptr = Module._malloc(width * height);
								var bytes = new Uint8ClampedArray(Module.HEAPU8.buffer, bytes_ptr, width * height);
								for (let i = 0; i < width * height; i++)
									bytes[i] = 0;

								console.log("Module._malloc bytes_ptr=", bytes_ptr);

								try {
									//init the HEVC encoder		
									api.hevc_init(va_count);
								} catch (e) { };

								//hevc decoding
								for (let i = 0; i < no_frames; i++) {
									let frame = frames[i];
									var len = frame.length;
									var ptr = Module._malloc(len);

									Module.HEAPU8.set(frame, ptr);

									try {
										//HEVC
										api.hevc_decode_nal_unit(0, ptr, len, null, width, height, null, bytes_ptr, "greyscale");
									} catch (e) { };

									Module._free(ptr);
								}

								try {
									api.hevc_destroy(va_count);
								} catch (e) { };

								process_viewport(width, height, width, height, bytes, width, alpha, index, false);

								Module._free(bytes_ptr);
							}
						}

						return;
					}

					//image
					if (type == 2) {
						var offset = 12;
						var id_length = dv.getUint32(offset, endianness);
						offset += 8;

						var identifier = new Uint8Array(received_msg, offset, id_length);
						identifier = new TextDecoder("utf-8").decode(identifier);
						offset += id_length;

						var width = dv.getUint32(offset, endianness);
						offset += 4;

						var height = dv.getUint32(offset, endianness);
						offset += 4;

						var image_length = dv.getUint32(offset, endianness);
						offset += 8;

						var frame = new Uint8Array(received_msg, offset, image_length);//offset by 8 bytes
						offset += image_length;

						var alpha_length = dv.getUint32(offset, endianness);
						offset += 8;

						var alpha = new Uint8Array(received_msg, offset);
						console.log("image frame identifier (WS): ", identifier, "width:", width, "height:", height, "compressed alpha length:", alpha.length);

						var Buffer = require('buffer').Buffer;
						var LZ4 = require('lz4');

						var uncompressed = new Buffer(width * height);
						uncompressedSize = LZ4.decodeBlock(new Buffer(alpha), uncompressed);
						alpha = uncompressed.slice(0, uncompressedSize);

						if (identifier == 'VP9') {
							var decoder = new OGVDecoderVideoVP9();

							decoder.init(function () { console.log("init callback done"); });
							decoder.processFrame(frame, function () {
								process_image(width, height, decoder.frameBuffer.format.displayWidth,
									decoder.frameBuffer.format.displayHeight,
									decoder.frameBuffer.y.bytes,
									decoder.frameBuffer.y.stride,
									alpha,
									index);
							});
						}

						return;

						//clear the Video Canvas
						/*var c = document.getElementById('VideoCanvas') ;
						var ctx = c.getContext("2d");
		
						var width = c.width ;
						var height = c.height ;
		    
						ctx.clearRect(0, 0, width, height);*/
					}

					//full spectrum refresh
					if (type == 3) {
						var length = dv.getUint32(12, endianness);
						var offset = 20;
						var mean_spectrum = new Float32Array(received_msg, offset, length);
						offset += 4 * length + 8;
						var integrated_spectrum = new Float32Array(received_msg, offset, length);

						/*self.postMessage;console.log({type: 'refresh', latency: latency, recv_seq_id: recv_seq_id, length: length, mean_spectrum: mean_spectrum, integrated_spectrum: integrated_spectrum});*/

						fitsContainer[index - 1].depth = length;
						fitsContainer[index - 1].mean_spectrum = mean_spectrum;
						fitsContainer[index - 1].integrated_spectrum = integrated_spectrum;

						//insert a spectrum object to the spectrumContainer at <index-1>
						mean_spectrumContainer[index - 1] = mean_spectrum;
						integrated_spectrumContainer[index - 1] = integrated_spectrum;

						spectrum_count++;

						if (va_count == 1) {
							setup_axes();

							if (intensity_mode == "mean")
								plot_spectrum([mean_spectrum]);

							if (intensity_mode == "integrated")
								plot_spectrum([integrated_spectrum]);
						}
						else {
							if (spectrum_count == va_count) {
								setup_axes();

								if (intensity_mode == "mean")
									plot_spectrum(mean_spectrumContainer);

								if (intensity_mode == "integrated")
									plot_spectrum(integrated_spectrumContainer);
							}
						}

						return;
					}

					//histogram refresh
					if (type == 4) {
						var min = dv.getFloat32(12, endianness);
						var max = dv.getFloat32(16, endianness);
						var black = dv.getFloat32(20, endianness);
						var white = dv.getFloat32(24, endianness);
						var median = dv.getFloat32(28, endianness);
						var sensitivity = dv.getFloat32(32, endianness);
						var ratio_sensitivity = dv.getFloat32(36, endianness);

						console.log("histogram refresh", min, max, median, sensitivity, ratio_sensitivity, black, white);

						let fitsData = fitsContainer[index - 1];
						console.log("min: ", fitsData.min, "-->", min);
						console.log("max: ", fitsData.max, "-->", max);
						console.log("median: ", fitsData.median, "-->", median);
						console.log("sensitivity: ", fitsData.sensitivity, "-->", sensitivity);
						console.log("ratio sensitivity: ", fitsData.ratio_sensitivity, "-->", ratio_sensitivity);
						console.log("black: ", fitsData.black, "-->", black);
						console.log("white: ", fitsData.white, "-->", white);

						fitsContainer[index - 1].min = min;
						fitsContainer[index - 1].max = max;
						fitsContainer[index - 1].median = median;
						fitsContainer[index - 1].sensitivity = sensitivity;
						fitsContainer[index - 1].ratio_sensitivity = ratio_sensitivity;
						fitsContainer[index - 1].black = black;
						fitsContainer[index - 1].white = white;

						var nbins = dv.getUint32(40, endianness);
						var histogram = new Int32Array(received_msg, 44, nbins);
						fitsContainer[index - 1].histogram = histogram;

						console.log("NBINS:", nbins, histogram);

						//refresh the histogram
						redraw_histogram(index);

						return;
					}

					//video
					if (type == 5) {
						computed = dv.getFloat32(12, endianness);

						var length = dv.getUint32(16, endianness);

						var frame = new Uint8Array(received_msg, 24);//16+8, extra 8 bytes for the length of the vector, added automatically by Rust

						var latency = performance.now() - dv.getFloat32(0, endianness);
						var transfer = (latency - computed) / 1000;//[s]

						if (transfer > 0) {
							var bandwidth = (received_msg.byteLength * 8 / 1000) / transfer;//[kilobits per s]

							//bitrate tracking (variance-tracking Kalman Filter)
							//eta = (variance - bitrate*bitrate) / (1 + Math.cosh(bitrate));
							bitrate = (1 - eta) * bitrate + eta * bandwidth;
							//variance = (1 - eta)*variance + eta * bandwidth*bandwidth;
							target_bitrate = 0.8 * bitrate;
						}

						console.log("[ws] computed = " + computed.toFixed(1) + " [ms], latency = " + latency.toFixed(1) + "[ms], n/w transfer time = " + (1000 * transfer).toFixed(1) + " [ms],  n/w bandwidth = " + Math.round(bandwidth) + " [kbps], frame length: " + length + " frame length:" + frame.length);

						//call the wasm decoder
						{
							let start = performance.now();

							var len = frame.length;
							var ptr = Module._malloc(len);

							Module.HEAPU8.set(frame, ptr);

							if (streaming && videoFrame[index - 1] != null && videoFrame[index - 1].img != null && videoFrame[index - 1].ptr != null && videoFrame[index - 1].alpha != null) {
								var img = videoFrame[index - 1].img;

								try {
									//VP9
									api.vpx_decode_frame(ptr, len, videoFrame[index - 1].ptr, img.width, img.height, videoFrame[index - 1].alpha, colourmap);
								} catch (e) { };

								try {
									//HEVC
									api.hevc_decode_nal_unit(index - 1, ptr, len, videoFrame[index - 1].ptr, img.width, img.height, videoFrame[index - 1].alpha, null, colourmap);
								} catch (e) { };

								if (img.data.length == 0) {
									//detect detached data due to WASM memory growth
									console.log("detached WASM buffer detected, refreshing videoFrame.ImageData");

									//WASM buffers have changed, need to refresh the ImageData.data buffer
									var len = img.width * img.height * 4;
									var data = new Uint8ClampedArray(Module.HEAPU8.buffer, videoFrame[index - 1].ptr, len);
									var img = new ImageData(data, img.width, img.height);

									videoFrame[index - 1].img = img;
								}

								requestAnimationFrame(function () {
									process_video(index)
								});
							}
							else {
								try {
									//VP9
									api.vpx_decode_frame(ptr, len, null, 0, 0, null, 'greyscale');
								} catch (e) { };

								try {
									//HEVC
									api.hevc_decode_nal_unit(index - 1, ptr, len, null, 0, 0, null, null, 'greyscale');
								} catch (e) { };
							}

							Module._free(ptr);

							let delta = performance.now() - start;

							console.log('total decoding/processing/rendering time: ' + delta.toFixed() + ' [ms]');

							let log = 'video frame length ' + len + ' bytes, decoding/processing/rendering time: ' + delta.toFixed() + ' [ms], bandwidth: ' + Math.round(bandwidth) + " [kbps], request latency: " + latency.toFixed() + ' [ms]';

							if (video_fps_control == 'auto') {
								//latency > computed or delta, take the greater
								if (Math.max(latency, delta) > 0.8 * vidInterval) {
									//reduce the video FPS
									vidFPS = 0.8 * vidFPS;
									vidFPS = Math.max(1, vidFPS);
								}
								else {
									//increase the video FPS
									vidFPS = 1.2 * vidFPS;
									vidFPS = Math.min(30, vidFPS);
								}
							}

							log += ' vidFPS = ' + Math.round(vidFPS);
							//wsConn[0].send('[debug] ' + log);

							if (videoFrame[index - 1] != null)
								d3.select("#fps").text('video: ' + Math.round(vidFPS) + ' fps, bitrate: ' + Math.round(bitrate) + ' kbps');//, η: ' + eta.toFixed(4) + ' var: ' + variance
						}

						return;
					}

					//CSV spectrum
					if (type == 6) {
						hide_hourglass();

						var csv_len = dv.getUint32(12, endianness);
						var csv_frame = new Uint8Array(received_msg, 16 + 8);

						// decompress CSV
						var LZ4 = require('lz4');

						var uncompressed = new Uint8Array(csv_len);
						uncompressedSize = LZ4.decodeBlock(csv_frame, uncompressed);
						uncompressed = uncompressed.slice(0, uncompressedSize);

						try {
							var csv = new TextDecoder().decode(uncompressed);

							// prepend the UTF-8 Byte Order Mark (BOM) 0xEF,0xBB,0xBF
							var blob = new Blob([new Uint8Array([0xEF, 0xBB, 0xBF]), csv], { type: "data:text/csv;charset=utf-8" });

							var filename;

							if (va_count == 1) {
								filename = datasetId + ".csv";
							} else {
								filename = datasetId[index - 1] + ".csv";
							};

							saveAs(blob, filename.replace('/', '_'));
						}
						catch (err) {
							console.error(err);
						};

						return;
					}
				}

				if (typeof evt.data === "string") {
					var cmd = "[close]";
					var pos = received_msg.indexOf(cmd);

					if (pos >= 0) {
						if (ALMAWS != null)
							ALMAWS.close();

						d3.select("#ping")
							.attr("fill", "red")
							.attr("opacity", 0.8);

						d3.select("#latency").text('60 min. inactive session time-out');

						show_timeout();

						return;
					}
				}

				if (typeof evt.data === "string") {
					var cmd = "[heartbeat]";
					var pos = received_msg.indexOf(cmd);

					if (pos >= 0) {
						setTimeout(send_ping, 1000 + ping_latency);

						var previous_t = parseFloat(received_msg.substring(pos + cmd.length));

						ping_latency = (t - previous_t);

						if (ping_latency > 0) {
							if (realtime_spectrum) {
								fps = 1000 / ping_latency;
								fps = Math.min(60, fps);
								fps = Math.max(10, fps);
							}
							else
								fps = 60;

							fpsInterval = 1000 / fps;
						}

						//console.log("ping latency = " + ping_latency.toFixed(1) + " [ms]" + ' fps: ' + fps.toFixed()) ;

						d3.select("#ping")
							.attr("fill", "green")
							.attr("opacity", 1.0);
						/*.transition()
						.duration(250)
						.attr("opacity", 0.0);*/

						if (ping_latency >= 1)
							d3.select("#latency").text('n/w latency: ' + ping_latency.toFixed() + ' ms' + ' ws: ' + fps.toFixed() + ' fps');
						else
							d3.select("#latency").text('n/w latency: ' + ping_latency.toFixed(1) + ' ms' + ' ws: ' + fps.toFixed() + ' fps');
						//d3.select("#latency").text('n/w latency: < 1 ms');			    

						return;
					}

					try {
						var data = JSON.parse(received_msg);

						if (data.type == "progress")
							process_progress_event(data, index);

						/*if (data.type == "image") {
							if (data.message.indexOf("unavailable") >= 0) {
								console.log("Server not ready, long-polling the image again after 100 ms.");
								setTimeout(function () { ALMAWS.send("[image]"); }, 100);
							}
						}*/

						if (data.type == "init_video") {
							var width = data.width;
							var height = data.height;
							var alpha = data.alpha;

							var Buffer = require('buffer').Buffer;
							var LZ4 = require('lz4');

							var uncompressed = new Buffer(width * height);
							uncompressedSize = LZ4.decodeBlock(new Buffer(alpha), uncompressed);
							alpha = uncompressed.slice(0, uncompressedSize);

							if (videoFrame[index - 1] == null) {
								let imageFrame = imageContainer[va_count - 1].imageFrame;

								var image_bounding_dims = true_image_dimensions(alpha, width, height);

								if (imageFrame != null) {
									var len = width * height * 4;
									var ptr = Module._malloc(len);

									var data = new Uint8ClampedArray(Module.HEAPU8.buffer, ptr, len);
									for (let i = 0; i < len; i++)
										data[i] = 0;
									var img = new ImageData(data, width, height);

									var alpha_ptr = Module._malloc(width * height);
									Module.HEAPU8.set(alpha, alpha_ptr);

									console.log("Module._malloc ptr=", ptr, "ImageData=", img, "alpha_ptr=", alpha_ptr);

									videoFrame[index - 1] = {
										img: img,
										ptr: ptr,
										alpha: alpha_ptr,
										scaleX: imageFrame.w / width,
										scaleY: imageFrame.h / height,
										image_bounding_dims: image_bounding_dims,
									}
								}
							}
						}

						return;
					}
					catch (e) {
						console.error(e);
					}
				}
			})

			wsConn[index - 1] = ALMAWS;
		}
	}
	else {
		d3.select("#welcome").append("p").text("LOADING IMAGE...");

		// The browser doesn't support WebSocket
		alert("WebSocket NOT supported by your Browser, progress updates disabled.");
	}
}

function fetch_binned_image(dataId) {
	var url = null;

	if (dataId >= "ALMA01000000")
		url = 'https://jvo.nao.ac.jp/portal/alma/archive.do?pictSize=512&dataId=' + dataId + '&dataType=image&action=quicklook';
	else
		url = 'https://jvo.nao.ac.jp/portal/alma/sv.do?pictSize=512&dataId=' + dataId + '&dataType=image&action=quicklook';

	var img = new Image();

	img.onload = function () {
		if (has_image)
			return;

		console.log("JVO Image Resolution: ", img.width, img.height);

		try {
			var c = document.getElementById('BackHTMLCanvas');
			var width = c.width;
			var height = c.height;
			var ctx = c.getContext("2d");

			ctx.mozImageSmoothingEnabled = false;
			ctx.webkitImageSmoothingEnabled = false;
			ctx.msImageSmoothingEnabled = false;
			ctx.imageSmoothingEnabled = false;

			var scale = get_image_scale(width, height, img.width, img.height);
			var img_width = scale * img.width;
			var img_height = scale * img.height;

			ctx.drawImage(img, (width - img_width) / 2, (height - img_height) / 2, img_width, img_height);
		}
		catch (e) { };
	};
	img.src = url;
}

function image_pixel_range(bytes, w, h, stride) {
	var min_pixel = 255;
	var max_pixel = 0;

	for (var j = 0; j < h; j++) {
		let offset = j * stride;

		for (var i = 0; i < w; i++) {
			let pixel = bytes[offset++];

			if (pixel > max_pixel)
				max_pixel = pixel;

			if (pixel < min_pixel)
				min_pixel = pixel;
		};
	};

	return { min_pixel: min_pixel, max_pixel: max_pixel };
}

function true_image_dimensions(alpha, width, height) {
	var width = width | 0;
	var height = height | 0;
	var linesize = width | 0;
	var length = (width * height) | 0;

	var x, y, offset;
	var found_data;

	var y1 = 0 | 0;
	var y2 = 0 | 0;
	var x1 = 0 | 0;
	var x2 = 0 | 0;

	//find y1
	for (var i = 0 | 0; i < length; i = (i + 1) | 0) {
		if (alpha[i] > 0) {
			y1 = (i / linesize) | 0;
			break;
		}
	}

	//find y2
	for (var i = length - 1; i >= 0; i = (i - 1) | 0) {
		if (alpha[i] > 0) {
			y2 = (i / linesize) | 0;
			break;
		}
	}

	//find x1
	found_data = false;
	for (var x = 0 | 0; x < width; x = (x + 1) | 0) {
		for (var y = y1; y <= y2; y = (y + 1) | 0) {
			if (alpha[y * linesize + x] > 0) {
				x1 = x | 0;
				found_data = true;
				break;
			}
		}

		if (found_data)
			break;
	}

	//find x2
	found_data = false;
	for (var x = (width - 1) | 0; x >= 0; x = (x - 1) | 0) {
		for (var y = y1; y <= y2; y = (y + 1) | 0) {
			if (alpha[y * linesize + x] > 0) {
				x2 = x | 0;
				found_data = true;
				break;
			}
		}

		if (found_data)
			break;
	}

	console.log("image bounding box: y1 =", y1, "y2 =", y2, "x1 =", x1, "x2 =", x2);

	return {
		x1: x1,
		y1: y1,
		width: Math.abs(x2 - x1) + 1,
		height: Math.abs(y2 - y1) + 1
	}
}

function display_hourglass() {
	var c = document.getElementById('HTMLCanvas');
	var width = c.width;
	var height = c.height;

	//hourglass
	/*var img_width = 200 ;
	var img_height = 200 ;*/

	//squares
	var img_width = 128;
	var img_height = 128;

	d3.select('#FrontSVG').append("svg:image")
		.attr("id", "hourglass")
		.attr("x", (width - img_width) / 2)
		.attr("y", (height - img_height) / 2)
		//.attr("xlink:href", ROOT_PATH + "loading.gif")
		.attr("xlink:href", "https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/loading.gif")
		.attr("width", img_width)
		.attr("height", img_height)
		.attr("opacity", 1.0);
}

function hide_hourglass() {
	try {
		d3.selectAll('#hourglass').remove();
	}
	catch (e) { };
}

function copy_coordinates(e) {
	var textToPutOnClipboard = d3.select("#ra").text() + " " + d3.select("#dec").text();

	if (ie) {
		window.clipboardData.setData('Text', textToPutOnClipboard);
	} else {
		e.clipboardData.setData('text/plain', textToPutOnClipboard);
	}
	e.preventDefault();
}

// Returns the inverse of matrix `M`.
function matrix_invert(M) {
	// I use Guassian Elimination to calculate the inverse:
	// (1) 'augment' the matrix (left) by the identity (on the right)
	// (2) Turn the matrix on the left into the identity by elemetry row ops
	// (3) The matrix on the right is the inverse (was the identity matrix)
	// There are 3 elemtary row ops: (I combine b and c in my code)
	// (a) Swap 2 rows
	// (b) Multiply a row by a scalar
	// (c) Add 2 rows

	//if the matrix isn't square: exit (error)
	if (M.length !== M[0].length) { return; }

	//create the identity matrix (I), and a copy (C) of the original
	var i = 0, ii = 0, j = 0, dim = M.length, e = 0, t = 0;
	var I = [], C = [];
	for (i = 0; i < dim; i += 1) {
		// Create the row
		I[I.length] = [];
		C[C.length] = [];
		for (j = 0; j < dim; j += 1) {

			//if we're on the diagonal, put a 1 (for identity)
			if (i == j) { I[i][j] = 1; }
			else { I[i][j] = 0; }

			// Also, make the copy of the original
			C[i][j] = M[i][j];
		}
	}

	// Perform elementary row operations
	for (i = 0; i < dim; i += 1) {
		// get the element e on the diagonal
		e = C[i][i];

		// if we have a 0 on the diagonal (we'll need to swap with a lower row)
		if (e == 0) {
			//look through every row below the i'th row
			for (ii = i + 1; ii < dim; ii += 1) {
				//if the ii'th row has a non-0 in the i'th col
				if (C[ii][i] != 0) {
					//it would make the diagonal have a non-0 so swap it
					for (j = 0; j < dim; j++) {
						e = C[i][j];       //temp store i'th row
						C[i][j] = C[ii][j];//replace i'th row by ii'th
						C[ii][j] = e;      //repace ii'th by temp
						e = I[i][j];       //temp store i'th row
						I[i][j] = I[ii][j];//replace i'th row by ii'th
						I[ii][j] = e;      //repace ii'th by temp
					}
					//don't bother checking other rows since we've swapped
					break;
				}
			}
			//get the new diagonal
			e = C[i][i];
			//if it's still 0, not invertable (error)
			if (e == 0) { return }
		}

		// Scale this row down by e (so we have a 1 on the diagonal)
		for (j = 0; j < dim; j++) {
			C[i][j] = C[i][j] / e; //apply to original matrix
			I[i][j] = I[i][j] / e; //apply to identity
		}

		// Subtract this row (scaled appropriately for each row) from ALL of
		// the other rows so that there will be 0's in this column in the
		// rows above and below this one
		for (ii = 0; ii < dim; ii++) {
			// Only apply to other rows (we want a 1 on the diagonal)
			if (ii == i) { continue; }

			// We want to change this element to 0
			e = C[ii][i];

			// Subtract (the row above(or below) scaled by e) from (the
			// current row) but start at the i'th column and assume all the
			// stuff left of diagonal is 0 (which it should be if we made this
			// algorithm correctly)
			for (j = 0; j < dim; j++) {
				C[ii][j] -= e * C[i][j]; //apply to original matrix
				I[ii][j] -= e * I[i][j]; //apply to identity
			}
		}
	}

	//we've done all operations, C should be the identity
	//matrix I should be the inverse:
	return I;
}

function inverse_CD_matrix(arcx, arcy) {
	let fitsData = fitsContainer[va_count - 1];

	if (fitsData == null)
		return;

	//convert from arc seconds to radians
	var dx = (arcx / 86400.0) * 2 * pi;//[s]
	var dy = (arcy / 3600.0) / toDegrees;//["]

	var CRPIX1 = fitsData.CRPIX1;
	var CRPIX2 = fitsData.CRPIX2;

	//convert to radians
	var CRVAL1 = fitsData.CRVAL1 / toDegrees;
	var CRVAL2 = fitsData.CRVAL2 / toDegrees;

	var RA = CRVAL1 + dx;
	var DEC = CRVAL2 + dy;

	//console.log(RadiansPrintHMS(CRVAL1), RadiansPrintHMS(RA)) ;
	//console.log(RadiansPrintDMS(CRVAL2), RadiansPrintDMS(DEC)) ;

	var y = (1 - Math.tan(CRVAL2) * Math.cos(dx) / Math.tan(DEC)) / (Math.tan(CRVAL2) + Math.cos(dx) / Math.tan(DEC));
	var x = Math.tan(dx) * Math.cos(CRVAL2) * (1 - y * Math.tan(CRVAL2));

	//convert from radians to degrees
	x = x * toDegrees;
	y = y * toDegrees;

	console.log("inverse: x = ", x, "y = ", y);

	var CD1_1 = fitsData.CD1_1;
	var CD1_2 = fitsData.CD1_2;
	var CD2_1 = fitsData.CD2_1;
	var CD2_2 = fitsData.CD2_2;

	//convert the North/East rotation angle from radians to degrees
	var theta = Math.atan(CD1_2 / CD1_1) * toDegrees;

	var M = [[CD1_1, CD1_2], [CD2_1, CD2_2]];
	var invM = matrix_invert(M);

	var DC1_1 = invM[0][0];
	var DC1_2 = invM[0][1];
	var DC2_1 = invM[1][0];
	var DC2_2 = invM[1][1];

	var DX = DC1_1 * x + DC1_2 * y;
	var DY = DC2_1 * x + DC2_2 * y;

	//DX: assume no change in y
	DX = DC1_1 * x;
	//DY: assume no change in x
	DY = DC2_2 * y;

	var gridScale = new Array(DX / fitsData.width, Math.sign(CD2_2) * Math.abs(DY) / fitsData.height, theta);

	return gridScale;
}

function CD_matrix(X, Y) {
	let fitsData = fitsContainer[va_count - 1];

	if (fitsData == null)
		return;

	var CRPIX1 = fitsData.CRPIX1;
	var CRPIX2 = fitsData.CRPIX2;

	var CRVAL1 = fitsData.CRVAL1;
	var CRVAL2 = fitsData.CRVAL2;

	//console.log(CRPIX1, CRVAL1, CRPIX2, CRVAL2) ;

	//convert to radians
	CRVAL1 = CRVAL1 / toDegrees;
	CRVAL2 = CRVAL2 / toDegrees;

	var CD1_1 = fitsData.CD1_1;
	var CD1_2 = fitsData.CD1_2;
	var CD2_1 = fitsData.CD2_1;
	var CD2_2 = fitsData.CD2_2;

	var x = CD1_1 * (X - CRPIX1) + CD1_2 * (Y - CRPIX2);
	var y = CD2_1 * (X - CRPIX1) + CD2_2 * (Y - CRPIX2);

	//convert to radians
	x = x / toDegrees;
	y = y / toDegrees;

	//console.log("x: ", x, "y: ", y, "X: ", X, "Y: ", Y) ;

	var a = Math.atan((x / Math.cos(CRVAL2)) / (1 - y * Math.tan(CRVAL2)));
	var ra = a + CRVAL1;
	var dec = Math.atan((y + Math.tan(CRVAL2)) * Math.cos(a) / (1 - y * Math.tan(CRVAL2)));

	var newradec = new Array(ra, dec);

	//console.log(RadiansPrintHMS(ra), RadiansPrintDMS(dec)) ;

	return newradec;
}

function x2hms(x) {
	let fitsData = fitsContainer[va_count - 1];

	if (fitsData == null)
		return "";

	if (fitsData.CDELT1 != null)
		return RadiansPrintHMS((fitsData.CRVAL1 + (x - fitsData.CRPIX1) * fitsData.CDELT1) / toDegrees);
	else
		throw "CDELT1 is not available";
};

function x2dms(x) {
	let fitsData = fitsContainer[va_count - 1];

	if (fitsData == null)
		return "";

	if (fitsData.CDELT1 != null)
		return RadiansPrintDMS((fitsData.CRVAL1 + (x - fitsData.CRPIX1) * fitsData.CDELT1) / toDegrees);
	else
		throw "CDELT1 is not available";
};

function y2dms(y) {
	let fitsData = fitsContainer[va_count - 1];

	if (fitsData == null)
		return "";

	if (fitsData.CDELT2 != null)
		return RadiansPrintDMS((fitsData.CRVAL2 + (fitsData.height - y - fitsData.CRPIX2) * fitsData.CDELT2) / toDegrees);
	else
		throw "CDELT2 is not available";
};

function display_scale_info() {
	let fitsData = fitsContainer[va_count - 1];

	if (fitsData == null)
		return;

	if (fitsData.depth > 1)
		return;

	var elem = d3.select("#image_rectangle");
	var img_width = parseFloat(elem.attr("width"));
	var img_height = parseFloat(elem.attr("height"));
	var img_x = parseFloat(elem.attr("x"));
	var img_y = parseFloat(elem.attr("y"));
	var image_bounding_dims = imageContainer[va_count - 1].image_bounding_dims;
	var imageCanvas = imageContainer[va_count - 1].imageCanvas;
	var scale = imageCanvas.height / image_bounding_dims.height;

	//scale
	var arcmins = 60;
	var gridScale = inverse_CD_matrix(arcmins, arcmins);

	for (let i = 0; i < gridScale.length; i++)
		if (isNaN(gridScale[i]))
			throw "NaN gridScale";

	if (Math.abs(gridScale[1]) * scale > 1) {
		//reduce the scale
		console.log("Vertical height:", Math.abs(gridScale[1]) * scale);

		arcmins = 10;
		gridScale = inverse_CD_matrix(arcmins, arcmins);

		for (let i = 0; i < gridScale.length; i++)
			if (isNaN(gridScale[i]))
				throw "NaN gridScale";

		console.log("Reduced vertical height:", Math.abs(gridScale[1]) * scale);
	}

	var svg = d3.select("#BackgroundSVG");
	var width = parseFloat(svg.attr("width"));
	var height = parseFloat(svg.attr("height"));

	var defs = svg.append("defs");

	defs.append("marker")
		.attr("id", "head")
		.attr("orient", "auto")
		.attr("markerWidth", (emStrokeWidth))
		.attr("markerHeight", (0.5 * emFontSize))
		.attr("refX", 0)
		.attr("refY", (0.5 * emFontSize / 2))
		.append("path")
		.style("stroke-width", 1)
		.attr("d", "M0,0 V" + 0.5 * emFontSize);

	defs.append("marker")
		.attr("id", "arrow")
		.attr("viewBox", "0 -5 10 10")
		.attr("refX", 5)
		.attr("refY", 0)
		.attr("markerWidth", 0.67 * emFontSize)
		.attr("markerHeight", 0.67 * emFontSize)
		.attr("orient", "auto")
		.append("path")
		.style("stroke-width", 1)
		.style("fill", "none")
		.attr("d", "M-5,-5 L5,0 L-5,5");

	//vertical scale	
	var L = Math.abs(gridScale[1]) * scale * img_height;
	var X = 1 * emFontSize;
	if (composite_view)
		X += img_x + img_width;
	//var Y = L + img_y;//1.75 * emFontSize;
	var Y = img_y + img_height;

	var vert = svg.append("g")
		.attr("id", "verticalScale");

	vert.append("path")
		.attr("marker-end", "url(#head)")
		.attr("marker-start", "url(#head)")
		.style("stroke-width", (emStrokeWidth))
		.style("fill", "none")
		.attr("d", "M" + X + "," + Y + " L" + X + "," + (Y - L));

	vert.append("text")
		.attr("x", (X + emFontSize))
		.attr("y", (Y - L / 2 + emFontSize / 3))
		.attr("font-family", "Monospace")
		.attr("font-size", "1.0em")
		.attr("text-anchor", "middle")
		.attr("stroke", "none")
		.text(arcmins + "\"");

	//N-E compass
	var L = 3 * emFontSize;//*Math.sign(gridScale[0]) ;
	var X = 0.02 * width + L + 1.5 * emFontSize;
	var Y = Y - L / 2;
	if (composite_view)
		X += img_x + img_width;
	//var Y = 0.01*width + L + emFontSize;
	//var Y = L + img_y;//Math.max(Y - 1.5 * emFontSize, 0.01 * width + L + emFontSize);

	//rotation
	var compass = svg.append("g")
		.attr("id", "compass")
		.attr("transform", 'rotate(' + gridScale[2] * Math.sign(gridScale[0]) + ' ' + X + ' ' + Y + ')');

	var east = compass.append("g")
		.attr("id", "east");

	east.append("path")
		.attr("marker-end", "url(#arrow)")
		.style("stroke-width", (emStrokeWidth))
		.style("fill", "none")
		.attr("d", "M" + X + "," + Y + " L" + (X + L * Math.sign(gridScale[0])) + "," + Y);

	east.append("text")
		.attr("x", (X + L * Math.sign(gridScale[0]) + Math.sign(gridScale[0]) * emFontSize / 2))
		.attr("y", (Y + emFontSize / 2.5))
		.attr("font-family", "Monospace")
		.attr("font-size", "1.0em")
		.attr("text-anchor", "middle")
		.attr("stroke", "none")
		.text("E");

	var north = compass.append("g")
		.attr("id", "north");

	L *= Math.sign(gridScale[1]);

	north.append("path")
		.attr("marker-end", "url(#arrow)")
		.style("stroke-width", (emStrokeWidth))
		.style("fill", "none")
		.attr("d", "M" + X + "," + Y + " L" + X + "," + (Y - L));

	if (L > 0)
		north.append("text")
			.attr("x", (X))
			.attr("y", (Y - L - emFontSize / 4))
			.attr("font-family", "Monospace")
			.attr("font-size", "1.1em")
			.attr("text-anchor", "middle")
			.attr("stroke", "none")
			.text("N");
	else
		north.append("text")
			.attr("x", (X))
			.attr("y", (Y - L + emFontSize))
			.attr("font-family", "Monospace")
			.attr("font-size", "1.0em")
			.attr("text-anchor", "middle")
			.attr("stroke", "none")
			.text("N");
}


function display_gridlines() {
	if (va_count > 1 && !composite_view)
		return;

	if (navigation == "static")
		return;

	let fitsData = fitsContainer[va_count - 1];

	if (fitsData == null)
		return;

	if (fitsData.CTYPE1.indexOf("RA") < 0 && fitsData.CTYPE1.indexOf("GLON") < 0 && fitsData.CTYPE1.indexOf("ELON") < 0) {
		d3.select("#displayGridlines")
			.style("font-style", "italic")
			.style('cursor', 'not-allowed')
			.style("display", "none")
			.attr("disabled", "disabled");
		return;
	}

	if (fitsData.CTYPE2.indexOf("DEC") < 0 && fitsData.CTYPE2.indexOf("GLAT") < 0 && fitsData.CTYPE2.indexOf("ELAT") < 0) {
		d3.select("#displayGridlines")
			.style("font-style", "italic")
			.style('cursor', 'not-allowed')
			.style("display", "none")
			.attr("disabled", "disabled");
		return;
	}

	if (!has_image)
		return;

	try {
		d3.select("#gridlines").remove();
	}
	catch (e) {
	}

	var elem = d3.select("#image_rectangle");
	var width = parseFloat(elem.attr("width"));
	var height = parseFloat(elem.attr("height"));

	var x_offset = parseFloat(elem.attr("x"));
	var y_offset = parseFloat(elem.attr("y"));

	var x = d3.scaleLinear()
		.range([x_offset, x_offset + width - 1])
		.domain([0, 1]);

	var y = d3.scaleLinear()
		.range([y_offset + height - 1, y_offset])
		.domain([1, 0]);

	var svg = d3.select("#BackgroundSVG");

	svg = svg.append("g")
		.attr("id", "gridlines")
		.attr("opacity", 1.0);

	let fillColour = 'white';
	let strokeColour = 'white';

	if (theme == 'bright') {
		fillColour = 'gray';
		strokeColour = 'gray';
	}

	if (colourmap == "greyscale" || colourmap == "negative") {
		fillColour = "#C4A000";
		strokeColour = fillColour;
	}

	// Add the X Axis
	if (fitsData.depth > 1) {
		var xAxis = d3.axisBottom(x)
			.tickSize(height)
			.tickFormat(function (d) {
				if (d == 0.0 || d == 1.0)
					return "";

				var image_bounding_dims = imageContainer[va_count - 1].image_bounding_dims;
				var imageCanvas = imageContainer[va_count - 1].imageCanvas;
				var tmp = image_bounding_dims.x1 + d * (image_bounding_dims.width - 1);
				var orig_x = tmp * fitsData.width / imageCanvas.width;

				try {
					if (fitsData.CTYPE1.indexOf("RA") > -1) {
						if (coordsFmt == 'DMS')
							return x2dms(orig_x);
						else
							return x2hms(orig_x);
					}

					if (fitsData.CTYPE1.indexOf("GLON") > -1 || fitsData.CTYPE1.indexOf("ELON") > -1)
						return x2dms(orig_x);
				}
				catch (err) {
					console.log(err);
				}

				return "";
			});

		svg.append("g")
			.attr("class", "gridlines")
			.attr("id", "ra_axis")
			.style("fill", fillColour)
			.style("stroke", strokeColour)
			.style("stroke-width", 1.0)
			.attr("opacity", 1.0)
			.attr("transform", "translate(0," + (y_offset) + ")")
			.call(xAxis)
			.selectAll("text")
			.attr("y", 0)
			.attr("x", 0)
			.style("fill", fillColour)
			.attr("dx", "-1.0em")
			.attr("dy", "1.0em")
			.attr("transform", "rotate(-45)")
			.style("text-anchor", "middle");
	}
	else {
		var xAxis = d3.axisTop(x)
			.tickSize(height)
			.tickFormat(function (d) {
				if (d == 0.0 || d == 1.0)
					return "";

				var image_bounding_dims = imageContainer[va_count - 1].image_bounding_dims;
				var imageCanvas = imageContainer[va_count - 1].imageCanvas;
				var tmp = image_bounding_dims.x1 + d * (image_bounding_dims.width - 1);
				var orig_x = tmp * fitsData.width / imageCanvas.width;

				try {
					if (fitsData.CTYPE1.indexOf("RA") > -1) {
						if (coordsFmt == 'DMS')
							return x2dms(orig_x);
						else
							return x2hms(orig_x);
					}

					if (fitsData.CTYPE1.indexOf("GLON") > -1 || fitsData.CTYPE1.indexOf("ELON") > -1)
						return x2dms(orig_x);
				}
				catch (err) {
					console.log(err);
				}

				return "";
			});

		svg.append("g")
			.attr("class", "gridlines")
			.attr("id", "ra_axis")
			.style("fill", fillColour)
			.style("stroke", strokeColour)
			.style("stroke-width", 1.0)
			.attr("opacity", 1.0)
			.attr("transform", "translate(0," + (height + y_offset) + ")")
			.call(xAxis)
			.selectAll("text")
			.attr("y", 0)
			.attr("x", 0)
			.style("fill", fillColour)
			//.attr("dx", ".35em")
			.attr("dy", ".35em")
			.attr("transform", "rotate(-45)")
			.style("text-anchor", "middle");
	}

	// Add the Y Axis
	/*if (!composite_view) {
		var yAxis = d3.axisRight(y)
			.tickSize(width)
			.tickFormat(function (d) {
				if (d == 0.0 || d == 1.0)
					return "";

				var image_bounding_dims = imageContainer[va_count - 1].image_bounding_dims;
				var imageCanvas = imageContainer[va_count - 1].imageCanvas;
				var tmp = image_bounding_dims.y1 + d * (image_bounding_dims.height - 1);
				var orig_y = tmp * fitsData.height / imageCanvas.height;

				try {
					return y2dms(orig_y);
				}
				catch (err) {
					console.log(err);
				}

				return "";
			});

		svg.append("g")
			.attr("class", "gridlines")
			.attr("id", "dec_axis")
			.style("fill", fillColour)
			.style("stroke", strokeColour)
			.style("stroke-width", 1.0)
			.attr("opacity", 1.0)
			.attr("transform", "translate(" + (x_offset) + ",0)")
			.call(yAxis)
			.selectAll("text")
			.attr("y", 0)
			.attr("x", 0)
			.style("fill", fillColour)
			.attr("dx", "-.35em")
			//.attr("dy", "-0.35em")
			//.attr("transform", "rotate(-45)")
			.style("text-anchor", "end");//was end, dx -.35, dy 0
	}
	else*/ {
		var yAxis = d3.axisLeft(y)
			.tickSize(width)
			.tickFormat(function (d) {
				if (d == 0.0 || d == 1.0)
					return "";

				var image_bounding_dims = imageContainer[va_count - 1].image_bounding_dims;
				var imageCanvas = imageContainer[va_count - 1].imageCanvas;
				var tmp = image_bounding_dims.y1 + d * (image_bounding_dims.height - 1);
				var orig_y = tmp * fitsData.height / imageCanvas.height;

				try {
					return y2dms(orig_y);
				}
				catch (err) {
					console.log(err);
				}

				return "";
			});

		svg.append("g")
			.attr("class", "gridlines")
			.attr("id", "dec_axis")
			.style("fill", fillColour)
			.style("stroke", strokeColour)
			.style("stroke-width", 1.0)
			.attr("opacity", 1.0)
			.attr("transform", "translate(" + (width + x_offset) + ",0)")
			.call(yAxis)
			.selectAll("text")
			.attr("y", 0)
			.attr("x", 0)
			.style("fill", fillColour)
			.attr("dx", ".35em")
			//.attr("dy", "-0.35em")
			//.attr("transform", "rotate(-45)")
			.style("text-anchor", "start");//was end, dx -.35, dy 0
	}

	if (va_count == 1 || composite_view) {
		var htmlStr = displayGridlines ? '<span class="fas fa-check-square"></span> lon/lat grid lines' : '<span class="far fa-square"></span> lon/lat grid lines';
		d3.select("#displayGridlines").html(htmlStr);

		var elem = d3.select("#gridlines");
		if (displayGridlines)
			elem.attr("opacity", 1);
		else
			elem.attr("opacity", 0);
	}
}

function display_cd_gridlines() {
	if (va_count > 1 && !composite_view)
		return;

	if (navigation == "static")
		return;

	let fitsData = fitsContainer[va_count - 1];

	if (fitsData == null)
		return;

	//scale
	var gridScale = inverse_CD_matrix(60, 60);//dx was 10
	var angle = gridScale[2] * Math.sign(gridScale[0]);

	var label_angle = -45;

	if (Math.sign(angle) != 0)
		label_angle *= Math.sign(angle);

	for (let i = 0; i < gridScale.length; i++)
		if (isNaN(gridScale[i]))
			throw "CD matrix is not available";

	if (fitsData.CTYPE1.indexOf("RA") < 0 && fitsData.CTYPE1.indexOf("GLON") < 0 && fitsData.CTYPE1.indexOf("ELON") < 0) {
		d3.select("#displayGridlines")
			.style("font-style", "italic")
			.style('cursor', 'not-allowed')
			.style("display", "none")
			.attr("disabled", "disabled");
		return;
	}

	if (fitsData.CTYPE2.indexOf("DEC") < 0 && fitsData.CTYPE2.indexOf("GLAT") < 0 && fitsData.CTYPE2.indexOf("ELAT") < 0) {
		d3.select("#displayGridlines")
			.style("font-style", "italic")
			.style('cursor', 'not-allowed')
			.style("display", "none")
			.attr("disabled", "disabled");
		return;
	}

	if (!has_image)
		return;

	try {
		d3.select("#gridlines").remove();
	}
	catch (e) {
	}

	var elem = d3.select("#image_rectangle");
	var width = parseFloat(elem.attr("width"));
	var height = parseFloat(elem.attr("height"));

	var x_offset = parseFloat(elem.attr("x"));
	var y_offset = parseFloat(elem.attr("y"));

	var x = d3.scaleLinear()
		.range([0, width - 1])
		.domain([-1, 1]);

	var y = d3.scaleLinear()
		.range([height - 1, 0])
		.domain([1, -1]);

	var svg = d3.select("#BackgroundSVG");

	svg = svg.append("g")
		.attr("id", "gridlines")
		.attr("opacity", 1.0);

	let fillColour = 'white';
	let strokeColour = 'white';

	if (theme == 'bright') {
		fillColour = 'gray';
		strokeColour = 'gray';
	}

	if (colourmap == "greyscale" || colourmap == "negative") {
		fillColour = "#C4A000";
		strokeColour = fillColour;
	}

	// Add the X Axis
	if (fitsData.depth > 1) {
		var xAxis = d3.axisBottom(x)
			.tickSize(height)
			.tickFormat(function (d) {
				if (d == -1.0 || d == 1.0)
					return "";

				var image_bounding_dims = imageContainer[va_count - 1].image_bounding_dims;
				var imageCanvas = imageContainer[va_count - 1].imageCanvas;

				var dx = d * Math.cos(angle / toDegrees);
				var dy = d * Math.sin(angle / toDegrees);

				//convert dx, dy to a 0 .. 1 range
				var tmpx = image_bounding_dims.x1 + (dx + 1) / 2 * (image_bounding_dims.width - 1);
				var tmpy = image_bounding_dims.y1 + (dy + 1) / 2 * (image_bounding_dims.height - 1);

				var orig_x = tmpx * fitsData.width / imageCanvas.width;
				var orig_y = tmpy * fitsData.height / imageCanvas.height;

				//use the CD scale matrix
				let radec = CD_matrix(orig_x, fitsData.height - orig_y);

				if (fitsData.CTYPE1.indexOf("RA") > -1) {
					if (coordsFmt == 'DMS')
						return RadiansPrintDMS(radec[0]);
					else
						return RadiansPrintHMS(radec[0]);
				}

				if (fitsData.CTYPE1.indexOf("GLON") > -1 || fitsData.CTYPE1.indexOf("ELON") > -1)
					return RadiansPrintDMS(radec[0]);

				return "";
			});

		svg.append("g")
			.attr("class", "gridlines")
			.attr("id", "ra_axis")
			.style("fill", fillColour)
			.style("stroke", strokeColour)
			.style("stroke-width", 1.0)
			.attr("opacity", 1.0)
			.attr("transform", "translate(" + (x_offset) + "," + (y_offset) + ")" + ' rotate(' + angle + ' ' + (width / 2) + ' ' + (height / 2) + ')')
			.call(xAxis)
			.selectAll("text")
			.attr("y", 0)
			.attr("x", 0)
			.style("fill", fillColour)
			//.attr("dx", "-1.0em")
			//.attr("dy", "1.0em")
			.attr("transform", "rotate(" + label_angle + ")")
			.style("text-anchor", "middle");
	}
	else {
		var xAxis = d3.axisTop(x)
			.tickSize(height)
			.tickFormat(function (d) {
				if (d == -1.0 || d == 1.0)
					return "";

				var image_bounding_dims = imageContainer[va_count - 1].image_bounding_dims;
				var imageCanvas = imageContainer[va_count - 1].imageCanvas;

				var dx = d * Math.cos(angle / toDegrees);
				var dy = d * Math.sin(angle / toDegrees);

				//convert dx, dy to a 0 .. 1 range
				var tmpx = image_bounding_dims.x1 + (dx + 1) / 2 * (image_bounding_dims.width - 1);
				var tmpy = image_bounding_dims.y1 + (dy + 1) / 2 * (image_bounding_dims.height - 1);

				var orig_x = tmpx * fitsData.width / imageCanvas.width;
				var orig_y = tmpy * fitsData.height / imageCanvas.height;

				//use the CD scale matrix
				let radec = CD_matrix(orig_x, fitsData.height - orig_y);

				if (fitsData.CTYPE1.indexOf("RA") > -1) {
					if (coordsFmt == 'DMS')
						return RadiansPrintDMS(radec[0]);
					else
						return RadiansPrintHMS(radec[0]);
				}

				if (fitsData.CTYPE1.indexOf("GLON") > -1 || fitsData.CTYPE1.indexOf("ELON") > -1)
					return RadiansPrintDMS(radec[0]);

				return "";
			});

		svg.append("g")
			.attr("class", "gridlines")
			.attr("id", "ra_axis")
			.style("fill", fillColour)
			.style("stroke", strokeColour)
			.style("stroke-width", 1.0)
			.attr("opacity", 1.0)
			.attr("transform", "translate(" + (x_offset) + "," + (height + y_offset) + ")" + ' rotate(' + angle + ' ' + (width / 2) + ' ' + (- height / 2) + ')')
			.call(xAxis)
			.selectAll("text")
			.attr("y", 0)
			.attr("x", 0)
			.style("fill", fillColour)
			//.attr("dx", ".35em")
			//.attr("dy", ".35em")			
			.attr("transform", "rotate(" + label_angle + ")")
			.style("text-anchor", "middle");
	}

	// Add the Y Axis
	/*if (!composite_view) {
		var yAxis = d3.axisRight(y)
			.tickSize(width)
			.tickFormat(function (d) {
				if (d == -1.0 || d == 1.0)
					return "";

				var image_bounding_dims = imageContainer[va_count - 1].image_bounding_dims;
				var imageCanvas = imageContainer[va_count - 1].imageCanvas;

				var dx = d * Math.sin(angle / toDegrees);
				var dy = d * Math.cos(angle / toDegrees);

				//convert dx, dy to a 0 .. 1 range
				var tmpx = image_bounding_dims.x1 + (dx + 1) / 2 * (image_bounding_dims.width - 1);
				var tmpy = image_bounding_dims.y1 + (dy + 1) / 2 * (image_bounding_dims.height - 1);

				var orig_x = tmpx * fitsData.width / imageCanvas.width;
				var orig_y = tmpy * fitsData.height / imageCanvas.height;

				//use the CD scale matrix
				let radec = CD_matrix(orig_x, fitsData.height - orig_y);

				if (fitsData.CTYPE2.indexOf("DEC") > -1 || fitsData.CTYPE2.indexOf("GLAT") > -1 || fitsData.CTYPE2.indexOf("ELAT") > -1)
					return RadiansPrintDMS(radec[1]);
				else return "";
			});

		svg.append("g")
			.attr("class", "gridlines")
			.attr("id", "dec_axis")
			.style("fill", fillColour)
			.style("stroke", strokeColour)
			.style("stroke-width", 1.0)
			.attr("opacity", 1.0)
			.attr("transform", " translate(" + (x_offset) + "," + (y_offset) + ")" + ' rotate(' + angle + ' ' + (width / 2) + ' ' + (height / 2) + ')')
			.call(yAxis)
			.selectAll("text")
			.attr("y", 0)
			.attr("x", 0)
			.style("fill", fillColour)
			.attr("dx", "-.35em")
			//.attr("dy", "-0.35em")
			.style("text-anchor", "end");//was end, dx -.35, dy 0
	}
	else*/ {
		var yAxis = d3.axisLeft(y)
			.tickSize(width)
			.tickFormat(function (d) {
				if (d == -1.0 || d == 1.0)
					return "";

				var image_bounding_dims = imageContainer[va_count - 1].image_bounding_dims;
				var imageCanvas = imageContainer[va_count - 1].imageCanvas;

				var dx = d * Math.sin(angle / toDegrees);
				var dy = d * Math.cos(angle / toDegrees);

				//convert dx, dy to a 0 .. 1 range
				var tmpx = image_bounding_dims.x1 + (dx + 1) / 2 * (image_bounding_dims.width - 1);
				var tmpy = image_bounding_dims.y1 + (dy + 1) / 2 * (image_bounding_dims.height - 1);

				var orig_x = tmpx * fitsData.width / imageCanvas.width;
				var orig_y = tmpy * fitsData.height / imageCanvas.height;

				//use the CD scale matrix
				let radec = CD_matrix(orig_x, fitsData.height - orig_y);

				if (fitsData.CTYPE2.indexOf("DEC") > -1 || fitsData.CTYPE2.indexOf("GLAT") > -1 || fitsData.CTYPE2.indexOf("ELAT") > -1)
					return RadiansPrintDMS(radec[1]);
				else return "";
			});

		svg.append("g")
			.attr("class", "gridlines")
			.attr("id", "dec_axis")
			.style("fill", fillColour)
			.style("stroke", strokeColour)
			.style("stroke-width", 1.0)
			.attr("opacity", 1.0)
			.attr("transform", " translate(" + (width + x_offset) + "," + (y_offset) + ")" + ' rotate(' + angle + ' ' + (- width / 2) + ' ' + (height / 2) + ')')
			.call(yAxis)
			.selectAll("text")
			.attr("y", 0)
			.attr("x", 0)
			.style("fill", fillColour)
			.attr("dx", ".35em")
			//.attr("dy", "-0.35em")
			//.attr("transform", "rotate(-45)")
			.style("text-anchor", "start");//was end, dx -.35, dy 0
	}

	if (va_count == 1 || composite_view) {
		var htmlStr = displayGridlines ? '<span class="fas fa-check-square"></span> lon/lat grid lines' : '<span class="far fa-square"></span> lon/lat grid lines';
		d3.select("#displayGridlines").html(htmlStr);

		var elem = d3.select("#gridlines");
		if (displayGridlines)
			elem.attr("opacity", 1);
		else
			elem.attr("opacity", 0);
	}
}

function display_beam() {
	if (va_count > 1 && !composite_view)
		return;

	if (optical_view)
		return;

	let fitsData = fitsContainer[va_count - 1];

	if (fitsData == null)
		return;

	if (!has_image)
		return;

	try {
		d3.select("#beam").remove();
	}
	catch (e) {
	}

	var elem = d3.select("#image_rectangle");
	var img_width = parseFloat(elem.attr("width"));
	var img_height = parseFloat(elem.attr("height"));
	var image_bounding_dims = imageContainer[va_count - 1].image_bounding_dims;
	var imageCanvas = imageContainer[va_count - 1].imageCanvas;
	var scale = (imageCanvas.width / fitsData.width) * (img_width / image_bounding_dims.width);

	//display telescope beam
	if (fitsData.BMIN > 0.0 && fitsData.BMAJ > 0.0) {
		var x_offset = parseFloat(elem.attr("x"));
		var y_offset = parseFloat(elem.attr("y"));

		var svg = d3.select("#BackgroundSVG");
		var width = parseFloat(svg.attr("width"));
		var height = parseFloat(svg.attr("height"));

		svg = svg.append("g")
			.attr("id", "beam")
			.attr("opacity", 1.0);

		var rx = 0.5 * scale * fitsData.BMAJ / Math.abs(fitsData.CDELT1);
		var ry = 0.5 * scale * fitsData.BMIN / Math.abs(fitsData.CDELT2);
		var max_dim = Math.max(rx, ry);
		var min_dim = Math.min(rx, ry);

		var beam_multiplier = 1.0;
		var beam_reduce = false;
		var beam_enlarge = false;
		var upper_threshold = 3 * emFontSize;
		var lower_threshold = 0.25 * emFontSize;

		while (max_dim > upper_threshold) {
			beam_reduce = true;
			beam_multiplier *= 2;
			max_dim /= beam_multiplier;

			console.log("beam_reduce scale multiplier = ", beam_multiplier);
		}

		if (!beam_reduce) {
			while (max_dim < lower_threshold) {
				beam_enlarge = true;
				beam_multiplier *= 2;
				max_dim *= beam_multiplier;

				console.log("beam_enlarge multiplier = ", beam_multiplier);
			}
		}

		if (beam_reduce) {
			rx /= beam_multiplier;
			ry /= beam_multiplier;
		}

		if (beam_enlarge) {
			rx *= beam_multiplier;
			ry *= beam_multiplier;
		}

		var rectSize = 2.5 * Math.max(rx, ry);

		console.log("rx:", rx, "ry:", ry);

		var cx = (width + img_width) / 2 - rectSize - 0.025 * img_width;
		var cy = (height - img_height) / 2 + rectSize + 0.025 * img_height;

		let fillColour = 'white';
		let strokeColour = 'white';

		if (theme == 'bright') {
			fillColour = 'black';
			strokeColour = 'black';
		}

		var beam = svg.append("ellipse")
			.attr("cx", cx)
			.attr("cy", cy)
			.attr("rx", rx)
			.attr("ry", ry)
			.attr("transform", "rotate(" + (-90 - fitsData.BPA) + ',' + cx + ',' + cy + ")")
			.attr("fill", fillColour)
			.attr("opacity", 1.0);

		rectSize = Math.max(rectSize, 0.0075 * width);//10

		var rect = svg.append("rect")
			.attr("x", cx - rectSize)
			.attr("y", cy - rectSize)
			.attr("width", 2 * rectSize)
			.attr("height", 2 * rectSize)
			.attr("fill", "none")
			.style("stroke", strokeColour)
			.attr("opacity", 1.0);

		if (beam_reduce) {
			svg.append("text")
				.attr("x", cx - rectSize + 0.5 * emFontSize)
				.attr("y", cy + rectSize - 0.5 * emFontSize)
				.attr("font-family", "Inconsolata")
				.attr("font-style", "italic")
				.attr("text-anchor", "start")
				.attr("stroke", "none")
				.text("reduced 1:" + beam_multiplier);
		}

		if (beam_enlarge) {
			svg.append("text")
				.attr("x", cx + rectSize + 0.0 * emFontSize)
				.attr("y", cy + rectSize + 1.0 * emFontSize)
				.attr("font-family", "Inconsolata")
				.attr("font-style", "italic")
				.attr("text-anchor", "end")
				.attr("stroke", "none")
				.text("enlarged " + beam_multiplier + ":1");
		}

		svg.moveToBack();

		displayBeam = true;
		var htmlStr = displayBeam ? '<span class="fas fa-check-square"></span> telescope beam' : '<span class="far fa-square"></span> telescope beam';
		d3.select("#displayBeam").html(htmlStr);
	}
}

function zoom_beam() {
	let fitsData = fitsContainer[va_count - 1];

	if (fitsData == null)
		return;

	if (!has_image)
		return;

	try {
		d3.select("#zoomBeam").remove();
	}
	catch (e) { };

	if (fitsData.BMIN > 0.0 && fitsData.BMAJ > 0.0) {
		var svg = d3.select("#BackSVG");

		var opacity = displayBeam ? 1 : 0;

		svg = svg.append("g")
			.attr("id", "zoomBeam")
			.attr("opacity", opacity);

		var image_bounding_dims = imageContainer[va_count - 1].image_bounding_dims;
		var imageCanvas = imageContainer[va_count - 1].imageCanvas;
		var clipSize = Math.min(image_bounding_dims.width, image_bounding_dims.height) / zoom_scale;
		var fitsSize = clipSize * fitsData.width / imageCanvas.width;

		var rx = 0.5 * fitsData.BMAJ / Math.abs(fitsData.CDELT1);
		var ry = 0.5 * fitsData.BMIN / Math.abs(fitsData.CDELT2);

		let strokeColour = 'white';

		if (theme == 'bright')
			strokeColour = 'black';

		//first handle the circular viewport
		if (zoom_shape == "circle") {
			var tmp, cx, cy, cr, scale;

			tmp = d3.select("#upper");
			cx = parseFloat(tmp.attr("cx"));
			cy = parseFloat(tmp.attr("cy"));
			cr = parseFloat(tmp.attr("r"));
			scale = cr / fitsSize;

			cx -= cr / 2;
			cy += cr / 2;

			if (Math.max(rx * scale, ry * scale) > cr)
				strokeColour = 'none';

			svg.append("ellipse")
				.attr("id", "upperBeam")
				.attr("cx", cx)
				.attr("cy", cy)
				.attr("rx", rx * scale)
				.attr("ry", ry * scale)
				.attr("transform", "rotate(" + (-90 - fitsData.BPA) + ',' + cx + ',' + cy + ")")
				.attr("fill", "none")
				.attr("stroke", strokeColour)
				.style("stroke-dasharray", ("5, 1, 5"))
				.attr("opacity", 0.0);

			tmp = d3.select("#lower");
			cx = parseFloat(tmp.attr("cx"));
			cy = parseFloat(tmp.attr("cy"));

			cx -= cr / 2;
			cy += cr / 2;

			svg.append("ellipse")
				.attr("id", "lowerBeam")
				.attr("cx", cx)
				.attr("cy", cy)
				.attr("rx", rx * scale)
				.attr("ry", ry * scale)
				.attr("transform", "rotate(" + (-90 - fitsData.BPA) + ',' + cx + ',' + cy + ")")
				.attr("fill", "none")
				.attr("stroke", strokeColour)
				.style("stroke-dasharray", ("5, 1, 5"))
				.attr("opacity", 0.0);
		}

		//next a square viewport
		if (zoom_shape == "square") {
			var tmp, cx, cy, cr, scale;
			var x, y, sizeX, sizeY;

			tmp = d3.select("#upper");
			x = parseFloat(tmp.attr("x"));
			y = parseFloat(tmp.attr("y"));
			sizeX = parseFloat(tmp.attr("width"));
			sizeY = parseFloat(tmp.attr("height"));

			cx = x + sizeX / 2;
			cy = y + sizeY / 2;

			cr = sizeX / 2;
			scale = cr / fitsSize;

			cx = cx - cr + cr / 4;
			cy = cy + cr - cr / 4;

			if (Math.max(rx * scale, ry * scale) > (sizeX / 2))
				strokeColour = 'none';

			svg.append("ellipse")
				.attr("id", "upperBeam")
				.attr("cx", cx)
				.attr("cy", cy)
				.attr("rx", rx * scale)
				.attr("ry", ry * scale)
				.attr("transform", "rotate(" + (-90 - fitsData.BPA) + ',' + cx + ',' + cy + ")")
				.attr("fill", "none")
				.attr("stroke", strokeColour)
				.style("stroke-dasharray", ("5, 1, 5"))
				.attr("opacity", 0.0);

			tmp = d3.select("#lower");
			x = parseFloat(tmp.attr("x"));
			y = parseFloat(tmp.attr("y"));
			sizeX = parseFloat(tmp.attr("width"));
			sizeY = parseFloat(tmp.attr("height"));

			cx = x + sizeX / 2;
			cy = y + sizeY / 2;

			cx = cx - cr + cr / 4;
			cy = cy + cr - cr / 4;

			svg.append("ellipse")
				.attr("id", "lowerBeam")
				.attr("cx", cx)
				.attr("cy", cy)
				.attr("rx", rx * scale)
				.attr("ry", ry * scale)
				.attr("transform", "rotate(" + (-90 - fitsData.BPA) + ',' + cx + ',' + cy + ")")
				.attr("fill", "none")
				.attr("stroke", strokeColour)
				.style("stroke-dasharray", ("5, 1, 5"))
				.attr("opacity", 0.0);
		}
	}
}

function frame_reference_unit(index) {
	let fitsData = fitsContainer[index - 1];

	if (fitsData.CUNIT3.toLowerCase() === "Hz".toLowerCase()) {
		has_frequency_info = true;
		frame_multiplier = 1;
		return;
	}

	if (fitsData.CUNIT3.toLowerCase() === "kHz".toLowerCase()) {
		has_frequency_info = true;
		frame_multiplier = 1e3;
		return;
	}

	if (fitsData.CUNIT3.toLowerCase() === "MHz".toLowerCase()) {
		has_frequency_info = true;
		frame_multiplier = 1e6;
		return;
	}

	if (fitsData.CUNIT3.toLowerCase() === "GHz".toLowerCase()) {
		has_frequency_info = true;
		frame_multiplier = 1e9;
		return;
	}

	if (fitsData.CUNIT3.toLowerCase() === "THz".toLowerCase()) {
		has_frequency_info = true;
		frame_multiplier = 1e12;
		return;
	}

	if (fitsData.CUNIT3.toLowerCase() === "m/s".toLowerCase()) {
		has_velocity_info = true;
		frame_multiplier = 1;
		return;
	}

	if (fitsData.CUNIT3.toLowerCase() === "km/s".toLowerCase()) {
		has_velocity_info = true;
		frame_multiplier = 1e3;
		return;
	}
}

function frame_reference_type(index) {
	let fitsData = fitsContainer[index - 1];

	if (fitsData.CTYPE3.toLowerCase().includes("f")) {
		has_frequency_info = true;
		return;
	}

	if (fitsData.CTYPE3.toLowerCase().includes("v")) {
		has_velocity_info = true;
		return;
	}
}

function display_dataset_info() {
	let fitsData = fitsContainer[va_count - 1];

	if (fitsData == null)
		return;

	var svg = d3.select("#FrontSVG");
	var width = parseFloat(svg.attr("width"));
	var height = parseFloat(svg.attr("height"));
	var maxoffset = 0;

	xradec = new Array(null, null);

	/*console.log("RA:", fitsData.OBSRA, fitsData.CTYPE1, "DEC:", fitsData.OBSDEC, fitsData.CTYPE2);

	if (fitsData.OBSRA != '' && fitsData.OBSDEC != '') {
		var ra = ParseRA('+' + fitsData.OBSRA.toString());
		var dec = ParseDec(fitsData.OBSDEC.toString());
		xradec = new Array((ra / 3600.0) / toDegrees, (dec / 3600.0) / toDegrees);
	}
	else
		xradec = new Array(null, null);*/

	if (fitsData.CTYPE1.indexOf("RA") > -1 || fitsData.CTYPE1.indexOf("GLON") > -1 || fitsData.CTYPE1.indexOf("ELON") > -1)
		xradec[0] = (fitsData.CRVAL1 + (fitsData.width / 2 - fitsData.CRPIX1) * fitsData.CDELT1) / toDegrees;

	if (fitsData.CTYPE2.indexOf("DEC") > -1 || fitsData.CTYPE2.indexOf("GLAT") > -1 || fitsData.CTYPE2.indexOf("ELAT") > -1)
		xradec[1] = (fitsData.CRVAL2 + (fitsData.height - fitsData.height / 2 - fitsData.CRPIX2) * fitsData.CDELT2) / toDegrees;

	try {
		d3.select("#information").remove();
	}
	catch (e) {
	}

	var group = svg.append("g")
		.attr("id", "information");

	var object = fitsData.OBJECT;
	var filter = fitsData.FILTER.trim().toUpperCase();

	/*if (object == '')
		object = 'OBJECT N/A';
	else*/ {
		//object = object.replace('_' + filter, '');//filter names are inconsistent!!!

		if (filter != "") {
			var pos = object.lastIndexOf('_');

			if (pos >= 0)
				object = object.substr(0, pos);
		}
	}

	var line = '';

	/*if(va_count == 1)
	{
	line = fitsData.LINE.trim() ;

	if(line != "")
		line = ' (' + line + ')' ;
	}*/

	maxoffset = 4.5 * emFontSize;
	group.append("text")
		.attr("x", width)
		.attr("y", maxoffset)//7*
		.attr("font-family", "Helvetica")//Arial
		.attr("font-weight", "normal")
		.attr("font-size", "2.5em")
		.attr("text-anchor", "end")
		.attr("stroke", "none")
		.text(object.replace(/_/g, " ") + line)
		.append("svg:title")
		.text("object name");

	document.title = 'FITSWebQL: ' + fitsData.OBJECT.replace(/_/g, " ");

	var dateobs = fitsData.DATEOBS;

	if (dateobs == '')
		dateobs = '';//'DATEOBS N/A' ;
	else {
		var pos = dateobs.indexOf('.');

		if (pos >= 0)
			dateobs = dateobs.substr(0, pos);

		dateobs = dateobs.replace(/T/g, " ") + ' ' + fitsData.TIMESYS;
	}

	maxoffset = 6.5 * emFontSize;
	group.append("text")
		.attr("x", width)
		.attr("y", maxoffset)//6.5 6.0
		.attr("font-family", "Helvetica")
		.attr("font-size", "1.3em")//1.75
		.attr("text-anchor", "end")
		.attr("stroke", "none")
		.attr("opacity", 0.75)
		.text(dateobs)
		.append("svg:title")
		.text("observation date");

	let raText = 'RA N/A';

	if (fitsData.CTYPE1.indexOf("RA") > -1) {
		if (coordsFmt == 'DMS')
			raText = 'α: ' + RadiansPrintDMS(xradec[0]);
		else
			raText = 'α: ' + RadiansPrintHMS(xradec[0]);
	}

	if (fitsData.CTYPE1.indexOf("GLON") > -1)
		raText = 'l: ' + RadiansPrintDMS(xradec[0]);

	if (fitsData.CTYPE1.indexOf("ELON") > -1)
		raText = 'λ: ' + RadiansPrintDMS(xradec[0]);

	maxoffset = 8.5 * emFontSize;
	group.append("text")
		.attr("id", "ra")
		.attr("x", width)
		.attr("y", maxoffset)//8.5 7.5
		.attr("font-family", "Inconsolata")
		//.attr("font-style", "italic")
		.attr("font-size", "1.5em")
		.attr("text-anchor", "end")
		.attr("stroke", "none")
		.text(raText);
	/*.append("svg:title")
	.text(fitsData.CTYPE1.trim());*/

	let decText = 'DEC N/A';

	if (fitsData.CTYPE2.indexOf("DEC") > -1)
		decText = 'δ: ' + RadiansPrintDMS(xradec[1]);

	if (fitsData.CTYPE2.indexOf("GLAT") > -1)
		decText = 'b: ' + RadiansPrintDMS(xradec[1]);

	if (fitsData.CTYPE2.indexOf("ELAT") > -1)
		decText = 'β: ' + RadiansPrintDMS(xradec[1]);

	maxoffset = 10 * emFontSize;
	group.append("text")
		.attr("id", "dec")
		.attr("x", width)
		.attr("y", maxoffset)//10 8.75
		.attr("font-family", "Inconsolata")
		//.attr("font-style", "italic")
		.attr("font-size", "1.5em")
		.attr("text-anchor", "end")
		.attr("stroke", "none")
		.text(decText);
	/*.append("svg:title")
	.text(fitsData.CTYPE2.trim());*/

	maxoffset = 11.5 * emFontSize;
	group.append("text")
		.attr("id", "pixel")
		.attr("x", width)
		.attr("y", maxoffset)//11.5 10
		.attr("font-family", "Inconsolata")
		//.attr("font-style", "italic")
		.attr("font-size", "1.5em")
		.attr("text-anchor", "end")
		.attr("stroke", "none")
		.attr("opacity", 0.0)
		.text("")
		.append("svg:title")
		.text("pixel value (intensity)");

	var val1 = fitsData.CRVAL3 + fitsData.CDELT3 * (1 - fitsData.CRPIX3);
	var val2 = fitsData.CRVAL3 + fitsData.CDELT3 * (fitsData.depth - fitsData.CRPIX3);

	data_band_lo = Math.min(val1, val2);
	data_band_hi = Math.max(val1, val2);
	RESTFRQ = fitsData.RESTFRQ;

	//disable frequency display in multiple-view mode
	if (va_count > 1)
		RESTFRQ = 0;
	else {
		if (RESTFRQ > 0.0)
			has_frequency_info = true;
	}

	if (has_velocity_info && has_frequency_info) {
		var c = 299792.458;//speed of light [km/s]

		var v1 = fitsData.CRVAL3 + fitsData.CDELT3 * (1 - fitsData.CRPIX3);
		v1 /= 1000;//[km/s]

		var v2 = fitsData.CRVAL3 + fitsData.CDELT3 * (fitsData.depth - fitsData.CRPIX3);
		v2 /= 1000;//[km/s]

		var f1 = RESTFRQ * Math.sqrt((1 - v1 / c) / (1 + v1 / c));
		var f2 = RESTFRQ * Math.sqrt((1 - v2 / c) / (1 + v2 / c));

		data_band_lo = Math.min(f1, f2);
		data_band_hi = Math.max(f1, f2);

		console.log("v1:", v1, "v2:", v2);
		console.log("f1:", f1, "f2:", f2);
	}
	else if (has_frequency_info) {
		/*if(fitsData.CTYPE3 != "")
		{	    
			//an override due to ALMA data errors: use the no middle-point
			RESTFRQ = (val1+val2)/2 ;//expects [Hz]	
		}*/

		RESTFRQ = (RESTFRQ / 1.0e9).toPrecision(7) * 1.0e9;//slightly rounded, expected unit is [Hz]

		data_band_lo = Math.min(val1, val2);
		data_band_hi = Math.max(val1, val2);
	}

	console.log("CTYPE3 = ", fitsData.CTYPE3, "has_freq:", has_frequency_info, "has_vel:", has_velocity_info);

	if (has_frequency_info > 0.0 && va_count == 1) {

		var bandStr = '';

		if (fitsData.depth > 1)
			bandStr = '<span style="float:left; font-weight:bold">REF FRQ</span><br><input type="number" id="frequencyInput" min="0" step="0.1" style="width: 6em; color: black; background-color: lightgray; font-size: 1.0em" value="' + (RESTFRQ / 1.0e9).toPrecision(7) + '"> GHz';
		else
			bandStr = '<span style="float:left; font-weight:bold">REF FRQ</span><br><input type="number" id="frequencyInput" min="0" step="0.1" style="width: 6em; color: black; background-color: lightgray; font-size: 1.0em" value="' + (RESTFRQ / 1.0e9).toPrecision(7) + '" disabled> GHz';

		maxoffset = 12.5 * emFontSize;
		group.append("g")
			.attr("id", "foreignBandG")
			.style("opacity", 0.25)
			.append("foreignObject")
			.attr("id", "foreignBand")
			.attr("x", (width - 20 * emFontSize))
			.attr("y", maxoffset)//12.5
			.attr("width", 20 * emFontSize)
			.attr("height", 7 * emFontSize)
			.on("mouseenter", function () {
				d3.select("#foreignBandG").style("opacity", 1.0);
			})
			.on("mouseleave", function () {
				d3.select("#foreignBandG").style("opacity", 0.25);
			})
			.append("xhtml:div")
			.attr("id", "bandDiv")
			.attr("class", "container-fluid input")
			.style("float", "right")
			.style("padding", "2.5%")
			.append("span")
			.attr("id", "band")
			.html(bandStr);

		var elem = document.getElementById('frequencyInput');
		elem.onblur = submit_corrections;
		elem.onmouseleave = submit_corrections;
		elem.onkeypress = function (e) {
			var event = e || window.event;
			var charCode = event.which || event.keyCode;

			if (charCode == '13') {
				console.log('REF FRQ ENTER');
				// Enter pressed
				submit_corrections();
				return false;
			}
		}
	}

	if (fitsData.depth > 1 && (/*has_velocity_info ||*/ has_frequency_info)) {
		var velStr = '<span id="redshift" class="redshift" style="float:left; font-weight:bold">SRC&nbsp;</span><input type="radio" id="velV" name="velocity" value="v" style="vertical-align: middle; margin: 0px;" onclick="javascript:toggle_redshift_input_source(this);"> V&nbsp;<input type="radio" id="velZ" name="velocity" value="z" style="vertical-align: middle; margin: 0px;" onclick="javascript:toggle_redshift_input_source(this);"> z&nbsp;<br><span><input type="number" id="velocityInput" step="0.1" style="width: 4.5em; color: black; background-color: lightgray; font-size: 1.0em" value="' + USER_DELTAV + '"></span> <span id="unit">km/s</span><br>';

		if (has_frequency_info)
			velStr += '<label class="small" style="cursor: pointer; font-weight:bold"><input type="checkbox" value="" class="control-label" style="cursor: pointer" id="restcheckbox" onmouseenter="javascript:this.focus();" onchange="javascript:toggle_rest_frequency();">&nbsp;<I>F<SUB>REST</SUB></I></label>'

		var yoffset = 21 * emFontSize;

		if (composite_view)
			yoffset += 1 * emFontSize;

		maxoffset = yoffset;
		group.append("g")
			.attr("id", "foreignVelG")
			.style("opacity", 0.25)
			.append("foreignObject")
			.attr("id", "foreignVel")
			.attr("x", (width - 20 * emFontSize))
			.attr("y", yoffset)//(17.5*emFontSize)//19
			.attr("width", 20 * emFontSize)
			.attr("height", 10.0 * emFontSize)
			.on("mouseenter", function () {
				d3.select("#foreignVelG").style("opacity", 1.0);
			})
			.on("mouseleave", function () {
				d3.select("#foreignVelG").style("opacity", 0.25);
			})
			.append("xhtml:div")
			.attr("id", "velDiv")
			.attr("class", "container-fluid input")
			.style("float", "right")
			.style("padding", "2.5%")
			.append("span")
			.attr("id", "vel")
			.html(velStr);

		if (has_frequency_info) {
			var checkbox = document.getElementById('restcheckbox');

			if (sessionStorage.getItem("rest") === null)
				checkbox.checked = false;
			else {
				var checked = sessionStorage.getItem("rest");

				if (checked == "true")
					checkbox.checked = true;
				else
					checkbox.checked = false;
			}
		}

		if (sessionStorage.getItem("redshift") === null)
			document.getElementById('velV').setAttribute("checked", "");
		else {
			var value = sessionStorage.getItem("redshift");

			if (value == "z") {
				document.getElementById('velZ').setAttribute("checked", "");
				document.getElementById('unit').style.opacity = "0.0";
			}
			else
				document.getElementById('velV').setAttribute("checked", "");
		}

		//add onblur
		var m = document.getElementById('velocityInput');
		m.onblur = submit_delta_v;
		m.onmouseleave = submit_delta_v;
		m.onkeypress = function (e) {
			var event = e || window.event;
			var charCode = event.which || event.keyCode;

			if (charCode == '13') {
				// Enter pressed
				submit_delta_v();
				return false;
			}
		}

	}

	//add video playback control
	if (fitsData.depth > 1) {
		var yoffset = 32 * emFontSize;

		if (composite_view)
			yoffset += 1 * emFontSize;

		var videoStr = '<span id="videoPlay" class="fas fa-play" style="display:inline-block; cursor: pointer"></span><span id="videoPause" class="fas fa-pause" style="display:none; cursor: pointer"></span>&nbsp; <span id="videoStop" class="fas fa-stop" style="cursor: pointer"></span>&nbsp; <span id="videoForward" class="fas fa-forward" style="cursor: pointer"></span>&nbsp; <span id="videoFastForward" class="fas fa-fast-forward" style="cursor: pointer"></span>';

		maxoffset = yoffset;
		group.append("g")
			.attr("id", "videoControlG")
			.style("opacity", 0.25)
			.append("foreignObject")
			.attr("id", "videoControl")
			.attr("x", (width - 20 * emFontSize))
			.attr("y", yoffset)//(17.5*emFontSize)//19
			.attr("width", 20 * emFontSize)
			.attr("height", 5.0 * emFontSize)
			.on("mouseenter", function () {
				d3.select("#videoControlG").style("opacity", 1.0);

				//hide the molecular list (spectral lines) so that it does not obscure the controls!
				displayMolecules_bak = displayMolecules;
				displayMolecules = false;
				document.getElementById('molecularlist').style.display = "none";
			})
			.on("mouseleave", function () {
				d3.select("#videoControlG").style("opacity", 0.25);

				displayMolecules = displayMolecules_bak;

				video_playback = false;
				clearTimeout(video_timeout);
				video_timeout = -1;

				document.getElementById('videoPlay').style.display = "inline-block";
				document.getElementById('videoPause').style.display = "none";

				if (streaming)
					x_axis_mouseleave();
			})
			.append("xhtml:div")
			.attr("id", "videoDiv")
			.attr("class", "container-fluid input")
			.style("float", "right")
			.style("padding", "2.5%")
			.append("span")
			.attr("id", "vel")
			.html(videoStr);

		document.getElementById('videoPlay').onclick = function () {
			video_playback = true;
			video_period = 10.0;

			if (video_offset == null)
				video_offset = [parseFloat(d3.select("#frequency").attr("x")), parseFloat(d3.select("#frequency").attr("y"))];

			document.getElementById('videoPlay').style.display = "none";
			document.getElementById('videoPause').style.display = "inline-block";

			if (!streaming)
				x_axis_mouseenter(video_offset);

			if (video_timeout < 0)
				replay_video();
		};

		document.getElementById('videoPause').onclick = function () {
			video_playback = false;
			clearTimeout(video_timeout);
			video_timeout = -1;

			document.getElementById('videoPlay').style.display = "inline-block";
			document.getElementById('videoPause').style.display = "none";
		};

		document.getElementById('videoStop').onclick = function () {
			video_playback = false;
			video_offset = null;
			clearTimeout(video_timeout);
			video_timeout = -1;

			document.getElementById('videoPlay').style.display = "inline-block";
			document.getElementById('videoPause').style.display = "none";

			if (streaming)
				x_axis_mouseleave();
		};

		document.getElementById('videoForward').onclick = function () {
			video_playback = true;
			video_period = 5.0;

			if (video_offset == null)
				video_offset = [parseFloat(d3.select("#frequency").attr("x")), parseFloat(d3.select("#frequency").attr("y"))];

			document.getElementById('videoPlay').style.display = "none";
			document.getElementById('videoPause').style.display = "inline-block";

			if (!streaming)
				x_axis_mouseenter(video_offset);

			if (video_timeout < 0)
				replay_video();
		};

		document.getElementById('videoFastForward').onclick = function () {
			video_playback = true;
			video_period = 2.5;

			if (video_offset == null)
				video_offset = [parseFloat(d3.select("#frequency").attr("x")), parseFloat(d3.select("#frequency").attr("y"))];

			document.getElementById('videoPlay').style.display = "none";
			document.getElementById('videoPause').style.display = "inline-block";

			if (!streaming)
				x_axis_mouseenter(video_offset);

			if (video_timeout < 0)
				replay_video();
		};
	}

	//add an image navigation switch
	/*if (va_count == 1) {		
		var yoffset = maxoffset;

		if (composite_view)
			yoffset += 1 * emFontSize;

		if (fitsData.depth > 1) {
			yoffset += 7 * emFontSize;
		}

		var nav_size = 5 * emFontSize;

		var resource = "";

		if (navigation == "dynamic")
			resource = "cursor.svg";

		if (navigation == "static")
			resource = "move.svg"

		svg.append("svg:image")
			.attr("id", "navigation")
			.attr("x", (width - nav_size))
			.attr("y", yoffset)
			.attr("xlink:href", ROOT_PATH + resource)//navigation-svgrepo-com.svg or navigation.svg
			//.attr("xlink:href", "https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/navigation.svg")
			.attr("width", nav_size)
			.attr("height", nav_size)
			.attr("opacity", 0.25)
			.style('cursor', 'pointer')
			.on("mouseenter", function () {
				d3.select(this).style("opacity", 1.0);
			})
			.on("mouseleave", function () {
				d3.select(this).style("opacity", 0.25);
			})
			.on("click", function () {
				//toggle the state
				if (navigation == "dynamic") {
					navigation = "static";
				} else
					if (navigation == "static") {
						navigation = "dynamic";
					}

				localStorage.setItem("navigation", navigation);
				console.log("image navigation: " + navigation);

				d3.select("#navigation_title")
					.text("current mode: " + navigation);

				location.reload();
			})
			.append("svg:title")
			.attr("id", "navigation_title")
			.text("image navigation: " + navigation);
	}*/

	var range = get_axes_range(width, height);

	svg.append("text")
		.attr("x", emFontSize / 4 /*width / 2*/)
		//.attr("y", 0.67 * range.yMin)
		.attr("y", 0.70 * range.yMin)
		.attr("font-family", "Helvetica")
		.attr("font-weight", "normal")
		//.attr("font-style", "italic")
		.attr("font-size", 0.75 * range.yMin)
		//.attr("text-anchor", "middle")
		.attr("stroke", "none")
		.attr("opacity", 0.5)//0.25
		//.text("☰ SETTINGS");
		//.text("⚙");
		.text("☰");

	let strokeColour = 'white';

	if (theme == 'bright')
		strokeColour = 'black';

	//add a menu activation area
	svg.append("rect")
		.attr("id", "menu_activation_area")
		.attr("x", 0/*emStrokeWidth*/)
		.attr("y", emStrokeWidth - 1)
		//.attr("width", (width - 2 * emStrokeWidth))
		.attr("width", (width))
		.attr("height", (range.yMin - 2 * emStrokeWidth))
		.attr("fill", "transparent")
		.attr("opacity", 0.1)//was 0.7
		.attr("stroke", strokeColour)//strokeColour or "transparent"
		.style("stroke-dasharray", ("1, 5"))
		.on("mouseenter", function () {
			d3.select(this).attr("opacity", 0);
			document.getElementById('menu').style.display = "block";
		});
}

function toggle_rest_frequency() {
	var checkbox = document.getElementById('restcheckbox');

	var freq_start = data_band_lo;
	var freq_end = data_band_hi;

	if (checkbox.checked) {
		sessionStorage.setItem("rest", "true");

		freq_start = relativistic_rest_frequency(freq_start);
		freq_end = relativistic_rest_frequency(freq_end);
	}
	else
		sessionStorage.setItem("rest", "false");

	//refresh spectral lines
	fetch_spectral_lines(datasetId, freq_start, freq_end);

	//refresh axes
	setup_axes();
}

function toggle_redshift_input_source(selection) {
	var unit = document.getElementById('unit');

	sessionStorage.setItem("redshift", selection.value);

	if (selection.value == "v")
		unit.style.opacity = "1.0";

	if (selection.value == "z")
		unit.style.opacity = "0.0";

	var m = document.getElementById('velocityInput');
	m.value = "0";
	m.focus();

	submit_delta_v();

	var m = document.getElementById('velocityInput');
	m.focus();

	console.log("toggled redshift input source");
}

function submit_delta_v() {
	//do we need to refresh molecules?
	if (has_frequency_info) {
		var checkbox = document.getElementById('restcheckbox');

		if (checkbox.checked) {
			var freq_start = relativistic_rest_frequency(data_band_lo);
			var freq_end = relativistic_rest_frequency(data_band_hi);

			//refresh spectral lines
			fetch_spectral_lines(datasetId, freq_start, freq_end);
		}
	}

	var strV = document.getElementById('velocityInput').value;

	if (strV.trim() == '')
		document.getElementById('velocityInput').value = 0;

	var tmp = 0.0;

	try {
		tmp = document.getElementById('velocityInput').valueAsNumber;
	}
	catch (e) {
		console.err(e);
	}

	//range checks
	var c = 299792.458;//speed of light [km/s]

	var value = sessionStorage.getItem("redshift");

	if (value == "z") {
		if (tmp <= -1) {
			document.getElementById('velocityInput').value = 0;
			invalid_range();
		}
		else
			USER_DELTAV = tmp;
	}
	else {
		if (tmp <= -c) {
			document.getElementById('velocityInput').value = 0;
			invalid_range();
		}
		else
			if (tmp >= c) {
				document.getElementById('velocityInput').value = 0;
				invalid_range();
			}
			else
				USER_DELTAV = tmp;

	};

	setup_axes();

	//re-attach lost event handlers
	{
		var m = document.getElementById('velocityInput');
		m.onblur = submit_delta_v;
		m.onmouseleave = submit_delta_v;
		m.onkeypress = function (e) {
			var event = e || window.event;
			var charCode = event.which || event.keyCode;

			if (charCode == '13') {
				// Enter pressed
				submit_delta_v();
				return false;
			}
		}
	}
}

function submit_corrections() {
	var referenceFrequency = document.getElementById('frequencyInput').valueAsNumber * 1e9;

	console.log("user referenceFrequency:", referenceFrequency);

	if (referenceFrequency > 0.0) {
		USER_SELFRQ = referenceFrequency;

		let fitsData = fitsContainer[va_count - 1];

		if (fitsData.RESTFRQ <= 0.0) {
			fitsContainer[va_count - 1].RESTFRQ = USER_SELFRQ;

			//has_frequency_info = false ;//force the re-creation of band ranges

			display_dataset_info();

			toggle_rest_frequency();
		}

		if (has_velocity_info && has_frequency_info) {
			fitsContainer[va_count - 1].RESTFRQ = USER_SELFRQ;

			display_dataset_info();

			toggle_rest_frequency();
		}
	}
	else
		USER_SELFRQ = RESTFRQ;

	set_user_restfrq();

	//re-attach lost event handlers
	{
		var elem = document.getElementById('frequencyInput');
		elem.onblur = submit_corrections;
		elem.onmouseleave = submit_corrections;
		elem.onkeypress = function (e) {
			var event = e || window.event;
			var charCode = event.which || event.keyCode;

			if (charCode == '13') {
				console.log('REF FRQ ENTER');
				// Enter pressed
				submit_corrections();
				return false;
			}
		}
	}
};

function validate_contour_lines() {
	var value = document.getElementById('contour_lines').valueAsNumber;

	if (isNaN(value) || value <= 0)
		document.getElementById('contour_lines').value = 10;

	value = document.getElementById('contour_lines').valueAsNumber;

	if (value != previous_contour_lines) {
		previous_contour_lines = value;
		update_contours();
	}
}

function set_user_restfrq() {
	RESTFRQ = USER_SELFRQ;

	var bandStr = bandStr = '<span style="float:left; font-weight:bold">REF FRQ</span><br><input type="number" id="frequencyInput" min="0" step="0.1" style="width: 6em; color: black; background-color: lightgray; font-size: 1.0em" value="' + (RESTFRQ / 1.0e9).toPrecision(7) + '"> GHz';

	d3.select("#band").html(bandStr);

	setup_axes();
};

function invalid_range() {
	$("#rangevalidation").modal("show");

	var modal = document.getElementById('rangevalidation');
	var span = document.getElementById('rangevalidationclose');

	// When the user clicks on <span> (x), close the modal
	span.onclick = function () {
		$("#rangevalidation").modal("hide");
	}
	// When the user clicks anywhere outside of the modal, close it
	window.onclick = function (event) {
		if (event.target == modal) {
			$("#rangevalidation").modal("hide");
		}
	}
}

function change_spectrum_scale(index) {
	var value = document.getElementById("scale" + index).value;

	spectrum_scale[index - 1] = parseFloat(value);

	change_intensity_mode();
}

function change_tone_mapping(index, recursive) {
	var display;

	if (document.getElementById('flux' + index).value == 'linear' || document.getElementById('flux' + index).value == 'log' || document.getElementById('flux' + index).value == 'square')
		display = "none";
	else
		display = "block";

	d3.select("#sensitivitySlider" + index)
		.style("display", display);

	noise_sensitivity = 50;
	document.getElementById('sensitivity' + index).value = noise_sensitivity;
	document.getElementById('sensitivityInput' + index).innerHTML = get_noise_sensitivity_string(noise_sensitivity, 2);

	setup_histogram_interaction(index);

	//request an image update from the server
	image_refresh(index);

	//change other datasets too
	if (va_count > 1 && recursive) {
		for (let i = 1; i <= va_count; i++)
			if (i != index) {
				document.getElementById('flux' + i).value = document.getElementById('flux' + index).value
				change_tone_mapping(i, false);
			}
	}
}

function image_refresh(index, refresh_histogram = true) {
	try {
		d3.selectAll('#contourPlot').remove();
	}
	catch (e) { };

	//has_contours = false ;

	if (refresh_histogram)
		enable_autoscale();

	/*displayContours = false ;
	var htmlStr = displayContours ? '<span class="fas fa-check-square"></span> contour lines' : '<span class="far fa-square"></span> contour lines' ;
	d3.select("#displayContours").html(htmlStr);*/

	var flux_elem = d3.select("#flux_path" + index);

	var black = '&black=';
	var white = '&white=';
	var median = '&median=';

	try {
		black += flux_elem.attr("black");
	}
	catch (e) {
	};

	try {
		white += flux_elem.attr("white");
	}
	catch (e) {
	};

	try {
		median += flux_elem.attr("median");
	}
	catch (e) {
	};

	var noise = '&noise=' + get_noise_sensitivity_string(noise_sensitivity, 3);
	var flux = '&flux=' + document.getElementById('flux' + index).value;
	var freq = '&frame_start=' + data_band_lo + '&frame_end=' + data_band_hi + '&ref_freq=' + RESTFRQ;
	var hist = '&hist=' + refresh_histogram;

	var strRequest = black + white + median + noise + flux + freq + hist;
	console.log(strRequest);

	//send an [image] request to the server    
	wsConn[index - 1].send('[image]' + strRequest + '&timestamp=' + performance.now());
}

function display_scale_range_ui(called_from_menu = false) {
	d3.select("#yaxis")
		.style("fill", 'white')
		.style("stroke", 'white')
		.transition()
		.duration(500)
		.style("fill", "#996699")
		.style("stroke", "#996699");

	var div = d3.select("body")
		.append("div")
		.attr("class", "container")
		.append("div")
		.attr("id", "scalingHelp")
		.attr("class", "modal")
		.attr("role", "dialog")
		.append("div")
		.attr("class", "modal-dialog");

	var content = div.append("div")
		.attr("class", "modal-content");

	var header = content.append("div")
		.attr("class", "modal-header");

	header.append("span")
		.attr("id", "scalingHeaderClose")
		.attr("class", "close")
		.style("color", "red")
		.text("×");

	header.append("h3")
		.text("How to Scale the Y-Axis");

	var body = content.append("div")
		.attr("class", "modal-body");

	body.append("p")
		.html("move mouse cursor over to the Y-Axis whilst holding the 「Shift」 key");

	body.append("p")
		.html("drag the mouse over the Y-Axis to <i>shift</i> it <em>UP</em> and <em>DOWN</em>");

	body.append("p")
		.html("use the mouse <i>scroll wheel</i> or a two-finger <i>touch gesture</i> to <i>re-scale</i> the Y-Axis range");

	var footer = content.append("div")
		.attr("class", "modal-footer");

	footer.append("p")
		.style("color", "#a94442")
		.html("you can disable showing this dialog via the <i>Preferences</i> menu, <i>display pop-up help</i> checkbox");

	if (called_from_menu)
		$('#scalingHelp').addClass("modal-center");

	if (displayScalingHelp) {
		show_scaling_help();
		$('#scalingHelp').modal('show');
	}

	/*var svg = d3.select("#FrontSVG") ;
	var width = parseFloat(svg.attr("width"));
	var height = parseFloat(svg.attr("height"));

	d3.select("#yaxis")
	.attr("data-toggle", "popover")
	.attr("data-trigger", "hover")
	.attr("title", "fixed scale")
	.attr("data-content", "hold 's' and move mouse over the Y-Axis, then use mouse drag/scroll-wheel to adjust the Y-Axis scale");

	$(document).ready(function(){
	$('[data-toggle="popover"]').popover();
	});*/

}

function set_autoscale_range(called_from_menu = false) {
	autoscale = false;
	var htmlStr = autoscale ? '<span class="fas fa-check-square"></span> autoscale y-axis' : '<span class="far fa-square"></span> autoscale y-axis';
	d3.select("#autoscale").html(htmlStr);

	user_data_min = tmp_data_min;
	user_data_max = tmp_data_max;

	plot_spectrum(last_spectrum);
	replot_y_axis();

	display_scale_range_ui(called_from_menu);
};

function enable_autoscale() {
	autoscale = true;
	var htmlStr = autoscale ? '<span class="fas fa-check-square"></span> autoscale y-axis' : '<span class="far fa-square"></span> autoscale y-axis';
	d3.select("#autoscale").html(htmlStr);

	user_data_min = null;
	user_data_max = null;
};

function change_video_fps_control() {
	video_fps_control = document.getElementById('video_fps_control').value;

	if (video_fps_control == 'auto')
		vidFPS = 5;//10
	else
		vidFPS = parseInt(video_fps_control);

	localStorage.setItem("video_fps_control", video_fps_control);
}

function change_zoom_shape() {
	zoom_shape = document.getElementById('zoom_shape').value;
	localStorage.setItem("zoom_shape", zoom_shape);

	if (navigation == "dynamic") {
		setup_image_selection();
		setup_viewports();
	}
}

function change_intensity_mode() {
	intensity_mode = document.getElementById('intensity_mode').value;
	localStorage.setItem("intensity_mode", intensity_mode);

	console.log("new intensity mode:", intensity_mode);

	let fitsData = fitsContainer[va_count - 1];

	if (fitsData != null) {
		if (fitsData.depth > 1) {
			if (va_count == 1) {
				if (intensity_mode == "mean") {
					data_min = d3.min(fitsData.mean_spectrum);
					data_max = d3.max(fitsData.mean_spectrum);

					plot_spectrum([fitsData.mean_spectrum]);
					replot_y_axis();
				}

				if (intensity_mode == "integrated") {
					data_min = d3.min(fitsData.integrated_spectrum);
					data_max = d3.max(fitsData.integrated_spectrum);

					plot_spectrum([fitsData.integrated_spectrum]);
					replot_y_axis();
				}
			}
			else {
				composite_data_min_max();

				if (intensity_mode == "mean") {
					plot_spectrum(mean_spectrumContainer);
					replot_y_axis();
				}

				if (intensity_mode == "integrated") {
					plot_spectrum(integrated_spectrumContainer);
					replot_y_axis();
				}
			}
		}
	}
}

function change_coords_fmt() {
	coordsFmt = document.getElementById('coords_fmt').value;
	localStorage.setItem("coordsFmt", coordsFmt);

	if (xradec != null) {
		let fitsData = fitsContainer[va_count - 1];

		if (fitsData.CTYPE1.indexOf("RA") > -1) {
			let raText = 'RA N/A';

			if (coordsFmt == 'DMS')
				raText = 'α: ' + RadiansPrintDMS(xradec[0]);
			else
				raText = 'α: ' + RadiansPrintHMS(xradec[0]);

			d3.select("#ra").text(raText);

			try {
				display_cd_gridlines();
			}
			catch (err) {
				display_gridlines();
			};
		}
	}
}

function change_ui_theme() {
	theme = document.getElementById('ui_theme').value;
	localStorage.setItem("ui_theme", theme);

	if (theme == 'bright')
		colourmap = "haxby";
	else
		colourmap = "green";

	localStorage.setItem("colourmap", colourmap);

	location.reload();
	//resizeMe() ;
}

function change_colourmap(index, recursive) {
	colourmap = document.getElementById('colourmap' + index).value;
	localStorage.setItem("colourmap", colourmap);

	var imageCanvas = imageContainer[index - 1].imageCanvas;
	var imageFrame = imageContainer[index - 1].imageFrame;
	var alpha = imageContainer[index - 1].alpha;
	var image_bounding_dims = imageContainer[index - 1].image_bounding_dims;

	if ((imageCanvas == null) || !has_image)
		return;

	var newImageCanvas = document.createElement('canvas');
	newImageCanvas.style.visibility = "hidden";

	newImageCanvas.width = imageCanvas.width;
	newImageCanvas.height = imageCanvas.height;

	var context = newImageCanvas.getContext('2d');

	var newImageData = context.createImageData(imageFrame.w, imageFrame.h);

	apply_colourmap(newImageData, colourmap, imageFrame.bytes, imageFrame.w, imageFrame.h, imageFrame.stride, alpha);

	context.putImageData(newImageData, 0, 0);

	imageContainer[index - 1].imageCanvas = newImageCanvas;
	imageContainer[index - 1].newImageData = newImageData;

	if (va_count == 1) {
		var c = document.getElementById('HTMLCanvas');
		var width = c.width;
		var height = c.height;
		var ctx = c.getContext("2d");

		ctx.mozImageSmoothingEnabled = false;
		ctx.webkitImageSmoothingEnabled = false;
		ctx.msImageSmoothingEnabled = false;
		ctx.imageSmoothingEnabled = false;

		var scale = get_image_scale(width, height, image_bounding_dims.width, image_bounding_dims.height);
		var img_width = scale * image_bounding_dims.width;
		var img_height = scale * image_bounding_dims.height;

		ctx.drawImage(newImageCanvas, image_bounding_dims.x1, image_bounding_dims.y1, image_bounding_dims.width, image_bounding_dims.height, (width - img_width) / 2, (height - img_height) / 2, img_width, img_height);

		if (navigation == "dynamic") {
			setup_image_selection();
			setup_viewports();
		}
		display_legend();
	}
	else
		if (!composite_view) {
			if (zoom_dims != null)
				if (zoom_dims.view != null)
					image_bounding_dims = zoom_dims.view;

			//place the image onto the main canvas
			var c = document.getElementById('HTMLCanvas' + index);
			var width = c.width;
			var height = c.height;
			var ctx = c.getContext("2d");

			ctx.mozImageSmoothingEnabled = false;
			ctx.webkitImageSmoothingEnabled = false;
			ctx.msImageSmoothingEnabled = false;
			ctx.imageSmoothingEnabled = false;
			//ctx.globalAlpha=0.9;

			var scale = get_image_scale(width, height, image_bounding_dims.width, image_bounding_dims.height);

			if (va_count == 2)
				scale = 0.8 * scale;
			else if (va_count == 4)
				scale = 0.6 * scale;
			else if (va_count == 5)
				scale = 0.5 * scale;
			else if (va_count == 6)
				scale = 0.45 * scale;
			else if (va_count == 7)
				scale = 0.45 * scale;
			else
				scale = 2 * scale / va_count;

			var img_width = scale * image_bounding_dims.width;
			var img_height = scale * image_bounding_dims.height;

			let image_position = get_image_position(index, width, height);
			let posx = image_position.posx;
			let posy = image_position.posy;

			ctx.drawImage(newImageCanvas, image_bounding_dims.x1, image_bounding_dims.y1, image_bounding_dims.width, image_bounding_dims.height, posx - img_width / 2, posy - img_height / 2, img_width, img_height);

			//add_line_label(index) ;

			//setup_image_selection_index(index, posx-img_width/2, posy-img_height/2, img_width, img_height);
		};

	let fitsData = fitsContainer[index - 1];

	if (fitsData != null) {
		if (fitsData.depth > 1) {
			if (va_count == 1) {
				if (intensity_mode == "mean") {
					plot_spectrum([fitsData.mean_spectrum]);
					replot_y_axis();
				}

				if (intensity_mode == "integrated") {
					plot_spectrum([fitsData.integrated_spectrum]);
					replot_y_axis();
				}
			}
			else {
				if (intensity_mode == "mean") {
					plot_spectrum(mean_spectrumContainer);
					replot_y_axis();
				}

				if (intensity_mode == "integrated") {
					plot_spectrum(integrated_spectrumContainer);
					replot_y_axis();
				}
			}
		}
	}

	//change other datasets too
	if (va_count > 1 && recursive) {
		for (let i = 1; i <= va_count; i++)
			if (i != index) {
				document.getElementById('colourmap' + i).value = colourmap;
				change_colourmap(i, false);
			}
	}

	//trigger a tileTimeout
	if (recursive) {
		if (zoom_dims != null)
			if (zoom_dims.view != null)
				tileTimeout(true);
	}
}

function add_histogram_line(g, pos, width, height, offset, info, position, addLine, index) {
	let fitsData = fitsContainer[index - 1];

	//slider size
	var side = 0.67 * emFontSize;
	//slider translation
	var d = [{ x: 0, width: width }];

	var min = fitsData.min;
	var max = fitsData.max;
	var x = (pos - min) / (max - min) * width;

	console.log(info, pos, width);

	var flux_elem = d3.select("#flux_path" + index);
	flux_elem.attr(info, pos);

	function dropGroup(d) {
		display_hourglass();

		if (!composite_view) {
			image_count = va_count - 1;

			image_refresh(index, false);
		}
		else {
			image_count = 0;

			for (let i = 1; i <= va_count; i++)
				image_refresh(i, false);
		}
	}

	function dragGroup(d) {
		d3.event.preventDefault = true;

		d.x += d3.event.dx;
		d.x = Math.max(-x, d.x);
		d.x = Math.min(width - x - 1, d.x);

		var black = (parseFloat(flux_elem.attr("black")) - min) / (max - min) * width;
		var white = (parseFloat(flux_elem.attr("white")) - min) / (max - min) * width;

		if (document.getElementById('flux' + index).value != "logistic" && document.getElementById('flux' + index).value != "ratio") {
			switch (info) {
				case 'black':
					d.x = Math.min(white - x - 1, d.x);
					break;

				case 'white':
					d.x = Math.max(black - x + 1, d.x);
					break;

				default:
					break;
			}
		}

		d3.select(this).attr("transform", "translate(" + d.x + "," + offset + ")");
		flux_elem.attr(info, ((x + d.x) / width * (max - min) + min));//transformed from pixels into server units

		var black, white, median;

		try {
			black = parseFloat(flux_elem.attr("black"));
		}
		catch (e) {
		};

		try {
			white = parseFloat(flux_elem.attr("white"));
		}
		catch (e) {
		};

		try {
			median = parseFloat(flux_elem.attr("median"));
		}
		catch (e) {
		};

		var multiplier = get_noise_sensitivity(noise_sensitivity);
		var path = get_flux_path(width, height, document.getElementById('flux' + index).value, black, white, median, multiplier, index);

		flux_elem.attr("d", path);

	}

	var group = g.data(d).append("g")
		.attr("id", info + "Group" + index)
		.style('cursor', 'move')
		.attr("transform", function (d) { return "translate(" + d.x + "," + offset + ")"; })
		.call(d3.drag()
			.on("drag", dragGroup)
			.on("end", dropGroup));

	if (addLine) {
		group.append("line")
			.attr("id", info + "Line" + index)
			.attr("x1", x)
			.attr("y1", (height - 1))
			.attr("x2", x)
			.attr("y2", 0)
			.style("stroke", "red")
			.style("stroke-width", emStrokeWidth)
			.style("stroke-dasharray", ("1, 5, 1"))
			.attr("opacity", 0.5);
	}

	var tHeight = (height - 1 - Math.sqrt(3) * side * 0.67);
	var points = "0,0 " + (2 * side) + ",0 " + (side) + "," + (-Math.sqrt(3) * side);

	if (position.includes('top')) {
		tHeight = (-Math.sqrt(3) * side * 0.33);
		points = "0," + (-Math.sqrt(3) * side) + " " + (2 * side) + "," + (-Math.sqrt(3) * side) + " " + (side) + ",0";
	}

	group.append("svg")
		.attr("id", info + "Slider" + index)
		.attr("x", (x - side))
		.attr("y", tHeight)
		.attr("viewBox", "0 " + (-Math.sqrt(3) * side) + " " + (2 * side) + " " + (Math.sqrt(3) * side))
		.attr("width", 2 * side)
		.attr("height", 2 * side)
		.append("polygon")
		.style("stroke", "none")
		.style("fill", "red")
		.attr("points", points);

	var txtHeight = (height - 1 - side);

	if (position.includes('top'))
		txtHeight = "1.25em";//0.75em

	if (position.includes('right')) {
		group.append("text")
			.attr("id", info + "Text" + index)
			.attr("x", (x + 0.5 * emFontSize))
			.attr("y", txtHeight)
			.attr("font-family", "Inconsolata")
			.attr("font-weight", "normal")
			.attr("font-style", "italic")
			.attr("font-size", "1em")
			.attr("text-anchor", "start")
			.attr("fill", "red")
			.attr("stroke", "none")
			.attr("opacity", 1.0)
			.text(info);
	}

	if (position.includes('left')) {
		group.append("text")
			.attr("id", info + "Text" + index)
			.attr("x", (x - 0.5 * emFontSize))
			.attr("y", txtHeight)
			.attr("font-family", "Inconsolata")
			.attr("font-weight", "normal")
			.attr("font-style", "italic")
			.attr("font-size", "1em")
			.attr("text-anchor", "end")
			.attr("fill", "red")
			.attr("stroke", "none")
			.attr("opacity", 1.0)
			.text(info);
	}
}

function get_pixel_flux(pixel, index) {
	var black, white, median, multiplier, flux;

	var flux_elem = d3.select("#flux_path" + index);

	try {
		flux = document.getElementById('flux' + index).value
	}
	catch (e) {
		console.log('flux not available yet');
		return NaN;
	};

	try {
		black = parseFloat(flux_elem.attr("black"));
	}
	catch (e) {
		console.log('black not available yet');
		return NaN;
	};

	try {
		white = parseFloat(flux_elem.attr("white"));
	}
	catch (e) {
		console.log('white not available yet');
		return NaN;
	};

	try {
		median = parseFloat(flux_elem.attr("median"));
	}
	catch (e) {
		console.log('median not available yet');
		return NaN;
	};

	multiplier = get_noise_sensitivity(noise_sensitivity);

	return get_flux(pixel / 255, flux, black, white, median, multiplier, index);
}

function get_flux_value_legacy(value, black, white, multiplier) {
	var p = get_slope_from_multiplier(multiplier);
	var lmin = Math.log(p);
	var lmax = Math.log(p + 1.0);

	var tmp = Math.exp(lmin + value * (lmax - lmin)) - p;
	return black + tmp * (white - black);
}

function get_flux_value_log(value, black, white) {
	return black + (Math.exp(value) - 1) / (Math.E - 1) * (white - black);
}

function get_flux_value_linear(value, black, white) {
	return black + value * (white - black);
}

function get_flux_value_logistic(value, min, max, median, sensitivity) {
	if (value == 0)
		return min;

	if (value == 1)
		return max;

	return median - Math.log(1 / value - 1) / (6 * sensitivity);
}

function get_flux_value_ratio(value, max, black, sensitivity) {
	if (value == 1)
		return max;

	return black + value / (5 * sensitivity * (1 - value));
}

function get_flux_value_square(value, black, white) {
	return black + Math.sqrt(value) * (white - black);
}

function get_flux(value, flux, black, white, median, multiplier, index) {
	let fitsData = fitsContainer[index - 1];
	let sensitivity = multiplier * fitsData.sensitivity;
	let ratio_sensitivity = multiplier * fitsData.ratio_sensitivity;
	var min = fitsData.min;
	var max = fitsData.max;

	switch (flux) {
		case 'linear':
			return get_flux_value_linear(value, black, white);
			break;
		case 'legacy':
			return get_flux_value_legacy(value, black, white, multiplier);
			break;
		case 'log':
			return get_flux_value_log(value, black, white);
			break;
		case 'logistic':
			return get_flux_value_logistic(value, min, max, median, sensitivity);
			break;
		case 'ratio':
			return get_flux_value_ratio(value, max, black, ratio_sensitivity);
			break;
		case 'square':
			return get_flux_value_square(value, black, white);
			break;
		default:
			return NaN;
			break;
	}
}

function get_flux_path_square(width, height, min, max, black, white, index) {
	let fitsData = fitsContainer[index - 1];
	var lower = min + black / width * (max - min);
	var upper = min + white / width * (max - min);

	var sensitivity = 1 / (upper - lower);
	var multiplier = sensitivity / fitsData.sensitivity;
	noise_sensitivity = get_noise_sensitivity_from_multiplier(multiplier);

	var path = "M0 " + (emStrokeWidth + height - 1) + " L" + black + " " + (emStrokeWidth + height - 1);

	var segments = 100;
	var dx = (white - black) / segments;

	for (var x = black; x < white + dx; x += dx) {
		var y = height - 1;
		var tmp = min + x / width * (max - min);
		tmp = (tmp - lower) * sensitivity;

		var pixel = tmp * tmp;
		pixel = Math.max(0.0, Math.min(1.0, pixel));

		y *= (1.0 - pixel);
		path += " L" + x + " " + (emStrokeWidth + y);
	}

	path += " L" + width + " " + emStrokeWidth;

	return path;
}

function get_flux_path_ratio(width, height, min, max, black, multiplier, index) {
	let fitsData = fitsContainer[index - 1];
	var sensitivity = multiplier * fitsData.ratio_sensitivity;
	var threshold = min + black / width * (max - min);

	var path = "M0 " + (emStrokeWidth + height - 1) + " L" + black + " " + (emStrokeWidth + height - 1);

	var segments = 100;
	var dx = (width - black) / segments;

	for (var x = black; x < width + dx; x += dx) {
		var y = height - 1;
		var tmp = min + x / width * (max - min);
		tmp = 5 * (tmp - threshold) * sensitivity;

		var pixel = tmp / (1 + tmp);
		pixel = Math.max(0.0, Math.min(1.0, pixel));

		y *= (1.0 - pixel);
		path += " L" + x + " " + (emStrokeWidth + y);
	}

	return path;
}

function get_flux_path_logistic(width, height, min, max, median, multiplier, index) {
	let fitsData = fitsContainer[index - 1];
	var sensitivity = multiplier * fitsData.sensitivity;
	var threshold = min + median / width * (max - min);

	var tmp = (min - threshold) * sensitivity;
	var pixel = 1.0 / (1.0 + Math.exp(-6 * tmp));
	var path = "M0 " + (emStrokeWidth + (height - 1) * (1.0 - pixel));

	var segments = 100;
	var dx = width / segments;

	for (var x = dx; x < width + dx; x += dx) {
		var y = height - 1;
		var tmp = min + x / width * (max - min);
		tmp = (tmp - threshold) * sensitivity;

		var pixel = 1.0 / (1.0 + Math.exp(-6 * tmp));
		pixel = Math.max(0.0, Math.min(1.0, pixel));

		y *= (1.0 - pixel);
		path += " L" + x + " " + (emStrokeWidth + y);
	}

	return path;
}

function get_flux_path_log(width, height, min, max, black, white, index) {
	let fitsData = fitsContainer[index - 1];
	var lower = min + black / width * (max - min);
	var upper = min + white / width * (max - min);

	var sensitivity = 0.5 * (Math.E - 1) / (upper - lower);
	var multiplier = sensitivity / fitsData.sensitivity;
	noise_sensitivity = get_noise_sensitivity_from_multiplier(multiplier);

	var path = "M0 " + (emStrokeWidth + height - 1) + " L" + black + " " + (emStrokeWidth + height - 1);

	var segments = 100;
	var dx = (white - black) / segments;

	for (var x = black; x < white + dx; x += dx) {
		var y = height - 1;
		var tmp = min + x / width * (max - min);
		tmp = (tmp - lower) * sensitivity;

		var pixel = (tmp > -0.5) ? Math.log(2.0 * tmp + 1.0) : 0.0;
		pixel = Math.max(0.0, Math.min(1.0, pixel));

		y *= (1.0 - pixel);
		path += " L" + x + " " + (emStrokeWidth + y);
	}

	path += " L" + width + " " + emStrokeWidth;

	return path;
}

function get_flux_path_legacy(width, height, black, white, multiplier) {
	var path = "M0 " + (emStrokeWidth + height - 1) + " L" + black + " " + (emStrokeWidth + height - 1);

	var segments = 100;
	var dx = (white - black) / segments;

	var p = get_slope_from_multiplier(multiplier);
	var lmin = Math.log(p);
	var lmax = Math.log(p + 1.0);//Math.log(1.5) ;

	//console.log("multiplier = ", multiplier, "p = ", p) ;

	for (var x = black; x < white + dx; x += dx) {
		var y = height - 1;
		var tmp = (x - black) / (white - black);

		//console.log("tmp = ", tmp) ;

		var pixel = (Math.log(p + tmp) - lmin) / (lmax - lmin);
		pixel = Math.max(0.0, Math.min(1.0, pixel));

		y *= (1.0 - pixel);
		path += " L" + x + " " + (emStrokeWidth + y);
	}

	path += " L" + width + " " + emStrokeWidth;

	return path;
}

function get_flux_path_linear(width, height, black, white) {
	var path = "M0 " + (emStrokeWidth + height - 1) + " L" + black + " " + (emStrokeWidth + height - 1);
	path += " L" + white + " " + emStrokeWidth;//0
	path += " L" + width + " " + emStrokeWidth;//0

	return path;
}

function get_flux_path(width, height, flux, black, white, median, multiplier, index) {
	let fitsData = fitsContainer[index - 1];
	var min = fitsData.min;
	var max = fitsData.max;

	var black = (black - min) / (max - min) * width;
	var white = (white - min) / (max - min) * width;
	var median = (median - min) / (max - min) * width;

	switch (flux) {
		case 'legacy':
			return get_flux_path_legacy(width, height, black, white, multiplier);
			break;

		case 'linear':
			return get_flux_path_linear(width, height, black, white);
			break;

		case 'log':
			return get_flux_path_log(width, height, min, max, black, white, index);
			break;

		case 'logistic':
			return get_flux_path_logistic(width, height, min, max, median, multiplier, index);
			break;

		case 'ratio':
			return get_flux_path_ratio(width, height, min, max, black, multiplier, index);
			break;

		case 'square':
			return get_flux_path_square(width, height, min, max, black, white, index);
			break;

		default:
			return "";
			break;
	};
}

function setup_histogram_interaction(index) {
	try {
		d3.select("#interaction" + index).remove();
	}
	catch (e) { };

	var c = document.getElementById("HistogramCanvas" + index);
	var svg = d3.select("#HistogramSVG" + index);

	var width = c.width;
	var height = c.height;
	var offset = parseFloat(svg.attr("offset"));

	var group = svg.append("g")
		.attr("id", "interaction" + index);

	let fitsData = fitsContainer[index - 1];

	console.log("min:", fitsData.min, "max:", fitsData.max, "median:", fitsData.median, "black:", fitsData.black, "white:", fitsData.white);

	var flux = document.getElementById('flux' + index).value;
	var min = fitsData.min;
	var max = fitsData.max;
	var black = fitsData.black;
	var white = fitsData.white;
	var median = fitsData.median;
	var multiplier = get_noise_sensitivity(noise_sensitivity);

	if (flux == 'legacy') {
		black = min;
		white = max;

		noise_sensitivity = 100;
		document.getElementById('sensitivity' + index).value = noise_sensitivity;
		document.getElementById('sensitivityInput' + index).innerHTML = get_noise_sensitivity_string(noise_sensitivity, 2);
		multiplier = get_noise_sensitivity(noise_sensitivity);
	}
	else {
		black = fitsData.black;
		white = fitsData.white;
	};

	var path = get_flux_path(width, height, flux, black, white, median, multiplier, index);

	group.append("path")
		.attr("id", "flux_path" + index)
		.attr("black", black)
		.attr("white", white)
		.attr("median", median)
		.attr("width", width)
		.attr("height", height)
		.attr("transform", "translate(0, " + offset + ")")
		.style("stroke", "red")
		.style("stroke-width", emStrokeWidth)
		.style("stroke-dasharray", ("3, 3, 1, 3"))
		.style("fill", "none")
		.attr("d", path);

	switch (flux) {
		case 'legacy':
			add_histogram_line(group, black, width, height, offset, 'black', 'right', false, index);
			add_histogram_line(group, white, width, height, offset, 'white', 'top left', true, index);
			break;

		case 'linear':
			add_histogram_line(group, black, width, height, offset, 'black', 'right', false, index);
			add_histogram_line(group, white, width, height, offset, 'white', 'top right', true, index);
			break;

		case 'log':
			add_histogram_line(group, black, width, height, offset, 'black', 'right', false, index);
			add_histogram_line(group, white, width, height, offset, 'white', 'top right', true, index);
			break;

		case 'ratio':
			add_histogram_line(group, black, width, height, offset, 'black', 'right', false, index);
			break;

		case 'square':
			add_histogram_line(group, black, width, height, offset, 'black', 'right', false, index);
			add_histogram_line(group, white, width, height, offset, 'white', 'top right', true, index);
			break;

		default:
			add_histogram_line(group, median, width, height, offset, 'median', 'right', true, index);
			break;
	};
}

function update_contours() {
	display_hourglass();

	setTimeout(function () {
		contour_surface();
		/*hide_hourglass() ;*/
	}, 0);//50, 100    
}

function redraw_histogram(index) {
	let fitsData = fitsContainer[index - 1];
	var histogram = fitsData.histogram;

	var c = document.getElementById("HistogramCanvas" + index);
	var ctx = c.getContext("2d");

	var width = c.width;
	var height = c.height;

	ctx.clearRect(0, 0, width, height);
	ctx.fillStyle = "rgba(0, 0, 0, 0.8)";

	var nbins = histogram.length;
	var dx = width / nbins;

	var dmin = d3.min(histogram);
	var dmax = d3.max(histogram);

	if (dmin > 0.0)
		dmin = Math.log(dmin);

	if (dmax > 0.0)
		dmax = Math.log(dmax);

	var binH;
	var binV;

	for (var i = 0; i < nbins; i++) {
		binV = histogram[i] > 0.0 ? Math.log(histogram[i]) : 0.0;
		binH = (binV - dmin) / (dmax - dmin) * (height - 1);
		ctx.fillRect(i * dx, height - 1, dx, -binH);

		if (histogram[i] > 0.0)
			ctx.fillRect(i * dx, height - 1, dx, height - 1);
	};

	setup_histogram_interaction(index);
}

function display_preferences(index) {
	if (has_preferences)
		return;

	let fitsData = fitsContainer[index - 1];//va_count

	if (fitsData == null) {
		console.log("display_preferences: NULL fitsData.");
		return;
	}
	else
		console.log("display_preferences: fitsData OK.");

	var svg = d3.select("#BackSVG");
	var svgWidth = parseFloat(svg.attr("width"));
	var svgHeight = parseFloat(svg.attr("height"));
	var offset = 2.0 * emFontSize;

	//show ping
	var group = svg.append("g")
		.attr("id", "pingGroup");

	group.append("text")
		.attr("id", "ping")
		.attr("x", "0.5em")
		//.attr("y", offset)//"0.75em")
		.attr("y", (svgHeight - offset / 4))
		.attr("font-family", "Helvetica")//Helvetica
		.attr("font-size", "0.75em")
		.attr("text-anchor", "start")
		.attr("fill", "green")
		.attr("stroke", "none")
		.attr("opacity", 1.0)
		//.text("■");
		.text("●");

	let fillColour = 'yellow';

	if (theme == 'bright')
		fillColour = 'black';

	group.append("text")
		.attr("id", "latency")
		.attr("x", "1.75em")
		//.attr("y", offset)//"0.85em")
		.attr("y", (svgHeight - offset / 4))
		.attr("font-family", "Inconsolata")
		//.attr("font-weight", "bold")
		.attr("font-size", "0.75em")//0.75 Helvetica
		.attr("text-anchor", "start")
		.attr("fill", fillColour)
		.attr("stroke", "none")
		.attr("opacity", 0.75)
		.text("");

	group.append("text")
		.attr("id", "fps")
		.attr("x", svgWidth)
		//.attr("y", offset)
		.attr("y", (svgHeight - offset / 4))
		.attr("font-family", "Inconsolata")
		//.attr("font-weight", "bold")
		.attr("font-size", "0.75em")//0.75 Helvetica
		.attr("text-anchor", "end")
		.attr("fill", fillColour)
		.attr("stroke", "none")
		.attr("opacity", 0.75)
		.text("");

	var prefDropdown = d3.select("#prefDropdown");

	var htmlStr = autoscale ? '<span class="fas fa-check-square"></span> autoscale y-axis' : '<span class="far fa-square"></span> autoscale y-axis';
	prefDropdown.append("li")
		.append("a")
		.attr("id", "autoscale")
		.style('cursor', 'pointer')
		.on("click", function () {
			autoscale = !autoscale;
			//localStorage_write_boolean("autoscale", autoscale) ;

			d3.select("#yaxis")
				.style("fill", "white")
				.style("stroke", "white")
				.transition()
				.duration(500)
				.style("fill", "#996699")
				.style("stroke", "#996699");

			if (!autoscale)
				set_autoscale_range(true);
			else
				enable_autoscale();
		})
		.html(htmlStr);

	var htmlStr = displayDownloadConfirmation ? '<span class="fas fa-check-square"></span> download confirmation' : '<span class="far fa-square"></span> download confirmation';
	prefDropdown.append("li")
		.append("a")
		.style('cursor', 'pointer')
		.on("click", function () {
			displayDownloadConfirmation = !displayDownloadConfirmation;
			localStorage_write_boolean("displayDownloadConfirmation", displayDownloadConfirmation);
			var htmlStr = displayDownloadConfirmation ? '<span class="fas fa-check-square"></span> download confirmation' : '<span class="far fa-square"></span> download confirmation';
			d3.select(this).html(htmlStr);
		})
		.html(htmlStr);

	var htmlStr = displayScalingHelp ? '<span class="fas fa-check-square"></span> display pop-up help' : '<span class="far fa-square"></span> display pop-up help';
	prefDropdown.append("li")
		.append("a")
		.style('cursor', 'pointer')
		.on("click", function () {
			displayScalingHelp = !displayScalingHelp;
			localStorage_write_boolean("displayScalingHelp", displayScalingHelp);
			var htmlStr = displayScalingHelp ? '<span class="fas fa-check-square"></span> display pop-up help' : '<span class="far fa-square"></span> display pop-up help';
			d3.select(this).html(htmlStr);
		})
		.html(htmlStr);

	var htmlStr = realtime_spectrum ? '<span class="fas fa-check-square"></span> realtime spectrum updates' : '<span class="far fa-square"></span> realtime spectrum updates';
	prefDropdown.append("li")
		.append("a")
		.style('cursor', 'pointer')
		.on("click", function () {
			realtime_spectrum = !realtime_spectrum;
			localStorage_write_boolean("realtime_spectrum", realtime_spectrum);
			var htmlStr = realtime_spectrum ? '<span class="fas fa-check-square"></span> realtime spectrum updates' : '<span class="far fa-square"></span> realtime spectrum updates';
			d3.select(this).html(htmlStr);
		})
		.html(htmlStr);

	var htmlStr = realtime_video ? '<span class="fas fa-check-square"></span> realtime video updates' : '<span class="far fa-square"></span> realtime video updates';
	prefDropdown.append("li")
		.append("a")
		.style('cursor', 'pointer')
		.on("click", function () {
			realtime_video = !realtime_video;
			localStorage_write_boolean("realtime_video", realtime_video);
			var htmlStr = realtime_video ? '<span class="fas fa-check-square"></span> realtime video updates' : '<span class="far fa-square"></span> realtime video updates';
			d3.select(this).html(htmlStr);

			if (realtime_video) {
				d3.select('#video_fps_control_li').style("display", "block");
			}
			else {
				d3.select('#video_fps_control_li').style("display", "none");
			}
		})
		.html(htmlStr);

	//----------------------------------------
	var tmpA;

	tmpA = prefDropdown.append("li")
		.attr("id", "video_fps_control_li")
		//.style("background-color", "#FFF")
		.append("a")
		.style("class", "form-group")
		.attr("class", "form-horizontal");

	tmpA.append("label")
		.attr("for", "video_fps_control")
		.attr("class", "control-label")
		.html("video fps control:&nbsp; ");

	tmpA.append("select")
		.attr("id", "video_fps_control")
		.attr("onchange", "javascript:change_video_fps_control();")
		.html("<option value='auto'>auto</option><option value='5'>5 fps</option><option value='10'>10 fps</option><option value='20'>20 fps</option><option value='30'>30 fps</option>");

	document.getElementById('video_fps_control').value = video_fps_control;

	if (realtime_video) {
		d3.select('#video_fps_control_li').style("display", "block");
	}
	else {
		d3.select('#video_fps_control_li').style("display", "none");
	}

	//ui_theme
	{
		tmpA = prefDropdown.append("li")
			//.style("background-color", "#FFF")	
			.append("a")
			.style("class", "form-group")
			.attr("class", "form-horizontal");

		tmpA.append("label")
			.attr("for", "ui_theme")
			.attr("class", "control-label")
			.html("ui theme:&nbsp; ");

		tmpA.append("select")
			//.attr("class", "form-control")	
			.attr("id", "ui_theme")
			.attr("onchange", "javascript:change_ui_theme();")
			.html("<option>dark</option><option>bright</option>");

		document.getElementById('ui_theme').value = theme;
	}

	//coords_fmt
	{
		tmpA = prefDropdown.append("li")
			//.style("background-color", "#FFF")	
			.append("a")
			.style("class", "form-group")
			.attr("class", "form-horizontal");

		tmpA.append("label")
			.attr("for", "coords_fmt")
			.attr("class", "control-label")
			.html("RA (<i>α</i>) display:&nbsp; ");

		tmpA.append("select")
			//.attr("class", "form-control")	
			.attr("id", "coords_fmt")
			.attr("onchange", "javascript:change_coords_fmt();")
			.html("<option>HMS</option><option>DMS</option>");

		document.getElementById('coords_fmt').value = coordsFmt;
	}

	tmpA = prefDropdown.append("li")
		.attr("id", "contour_control_li")
		//.style("background-color", "#FFF")
		.append("a")
		.style("class", "form-group")
		.attr("class", "form-horizontal");

	tmpA.append("label")
		.attr("for", "contour_lines")
		.attr("class", "control-label")
		.html("#contour levels:&nbsp; ");

	previous_contour_lines = 5;

	tmpA.append("input")
		//.attr("class", "form-control")	
		.attr("id", "contour_lines")
		.attr("type", "number")
		.style("width", "3em")
		.attr("min", 1)
		.attr("step", 1)
		.attr("value", previous_contour_lines);
	//.attr("onchange", "javascript:update_contours();");    

	var elem = document.getElementById('contour_lines');
	elem.onblur = validate_contour_lines;
	elem.onmouseleave = validate_contour_lines;
	elem.onkeypress = function (e) {
		var event = e || window.event;
		var charCode = event.which || event.keyCode;

		if (charCode == '13') {
			// Enter pressed
			validate_contour_lines();
			return false;
		}
	}

	if (displayContours) {
		d3.select('#contour_control_li').style("display", "block");
	}
	else {
		d3.select('#contour_control_li').style("display", "none");
	}

	//----------------------------------------
	if (fitsData.depth > 1) {
		tmpA = prefDropdown.append("li")
			//.style("background-color", "#FFF")
			.append("a")
			.style("class", "form-group")
			.attr("class", "form-horizontal");

		tmpA.append("label")
			.attr("for", "intensity_mode")
			.attr("class", "control-label")
			.html("intensity mode:&nbsp; ");

		tmpA.append("select")
			.attr("id", "intensity_mode")
			.attr("onchange", "javascript:change_intensity_mode();")
			.html("<option>mean</option><option>integrated</option>");

		document.getElementById('intensity_mode').value = intensity_mode;
	}

	tmpA = prefDropdown.append("li")
		//.style("background-color", "#FFF")	
		.append("a")
		.style("class", "form-group")
		.attr("class", "form-horizontal");

	tmpA.append("label")
		.attr("for", "zoom_shape")
		.attr("class", "control-label")
		.html("zoom shape:&nbsp; ");

	tmpA.append("select")
		//.attr("class", "form-control")	
		.attr("id", "zoom_shape")
		.attr("onchange", "javascript:change_zoom_shape();")
		.html("<option>circle</option><option>square</option>");

	document.getElementById('zoom_shape').value = zoom_shape;
	//----------------------------------------

	has_preferences = true;
}

function display_histogram(index) {
	let fitsData = fitsContainer[index - 1];//va_count

	if (fitsData == null) {
		console.log("display_histogram: NULL fitsData.");
		return;
	}
	else
		console.log("display_histogram: fitsData OK.");

	var imageDropdown = d3.select("#imageDropdown");

	//add multiple panes
	if (va_count > 1) {
		if ($("#imageDropdown li").length == 0) {
			var ul = imageDropdown
				.append("li")
				.append("ul")
				.attr("class", "nav nav-tabs")
				.style("background-color", "#FFF");

			for (let index = 1; index <= va_count; index++) {
				let classStr = '';

				if (index == 1)
					classStr = 'active';

				var li = ul.append("li")
					.attr("class", classStr);

				var a = li.append("a")
					.attr("id", "imageTag#" + index)
					.attr("data-toggle", "tab")
					.attr("href", "#image" + index)
					.style("font-weight", "bold")
					.html(datasetId[index - 1]);
			}

			var div = imageDropdown.append("li")
				.append("div")
				.attr("class", "tab-content form-group");

			for (let index = 1; index <= va_count; index++) {
				let classStr = 'tab-pane fade';

				if (index == 1)
					classStr += ' in active';

				var tab = div.append("div")
					.attr("id", "image" + index)
					.attr("class", classStr);
			}
		}

		imageDropdown = d3.select("#image" + index)
			.append("ul")
			.style("list-style", "none")
			.style("list-style-type", "none")
			.style("list-style-position", "outside")
			.style("list-style-image", "none")
			.style("text-align", "left")
			.style("padding-left", "0");
	}

	var colourmap_string = "<option>red</option><option>green</option><option>blue</option><option>greyscale</option><option>negative</option><option disabled>---</option><option>cubehelix</option><option>haxby</option><option>hot</option><option>rainbow</option><option>parula</option><option disabled>---</option><option>inferno</option><option>magma</option><option>plasma</option><option>viridis</option>";

	tmpA = imageDropdown.append("li")
		//.style("background-color", "#FFF")
		.append("a")
		.style("class", "form-group")
		.attr("class", "form-horizontal custom");

	if (!composite_view) {
		tmpA.append("label")
			.attr("for", "colourmap" + index)
			.attr("class", "control-label")
			.html("colourmap:&nbsp; ");

		tmpA.append("select")
			.attr("id", "colourmap" + index)
			.attr("onchange", "javascript:change_colourmap(" + index + ",true);")
			.html(colourmap_string);

		document.getElementById('colourmap' + index).value = colourmap;
	}

	tmpA = imageDropdown.append("li")
		//.style("background-color", "#FFF")
		.append("a")
		.attr("class", "form-group")
		.attr("class", "form-horizontal custom");

	tmpA.append("label")
		.attr("for", "flux" + index)
		.attr("class", "control-label")
		.html("tone mapping:&nbsp; ");

	tmpA.append("select")
		//.attr("class", "form-control")
		.attr("id", "flux" + index)
		.attr("onchange", "javascript:image_count=0;display_hourglass();change_tone_mapping(" + index + ",true);")
		.html("<option value='linear'>linear</option><option value='legacy'>logarithmic</option><option value='logistic'>logistic</option><option value='ratio'>ratio</option><option value='square'>square</option>");//<option value='log'>log</option>

	document.getElementById('flux' + index).value = fitsData.flux;
	//document.querySelectorAll('[value="' + fitsData.flux + '"]')[0].text = fitsData.flux + ' (default)' ;

	var display;

	if (fitsData.flux == 'linear' || fitsData.flux == 'log' || fitsData.flux == 'square')
		display = "none";
	else
		display = "block";

	tmpA = imageDropdown.append("li")
		//.style("background-color", "#FFF")
		.append("a")
		.attr("id", "sensitivitySlider" + index)
		.attr("class", "form-group")
		.style("display", display)
		.attr("class", "form-horizontal custom");

	tmpA.append("label")
		.attr("for", "sensitivity" + index)
		.attr("class", "control-label")
		.html('image noise sensitivity:&nbsp; <span id="sensitivityInput' + index + '">' + get_noise_sensitivity_string(noise_sensitivity, 2) + "</span>");

	tmpA.append("input")
		//.attr("class", "form-control")
		.attr("id", "sensitivity" + index)
		.attr("class", "slider")
		.attr("type", "range")
		.attr("min", "0")
		.attr("max", "100")
		.attr("step", "1")
		.attr("value", noise_sensitivity)
		.attr("onmousemove", "javascript:change_noise_sensitivity(false," + index + ");")
		.attr("onchange", "javascript:change_noise_sensitivity(true," + index + ");");

	var mainRect = document.getElementById('mainDiv').getBoundingClientRect();
	var width = 0.33 * mainRect.width;
	var height = width / (1 + Math.sqrt(2));

	//histogram part
	var histLI = imageDropdown.append("li");

	var histDiv = histLI.append("div")
		.attr("id", "histogram" + index)
		.attr('style', 'width:' + (width + 2 * emFontSize) + 'px; height:' + (height) + 'px;');
	//.style("width", (width+2*emFontSize))
	//.style("height", height);
	//.attr('style', 'position: fixed');

	var histWidth = width - 2 * emFontSize;//0.75
	var histHeight = height - 2 * emFontSize;
	var svgWidth = histWidth;
	var svgHeight = height;

	histDiv.append("canvas")
		.attr("id", "HistogramCanvas" + index)
		.attr("width", histWidth)
		.attr("height", histHeight)
		.attr('style', 'position: relative; left: 1em; top: 1em;');
	//.style("background-color", "#FFF")
	//.style("background-color", "rgba(0,0,0,0.4)");    

	histDiv.append("svg")
		.attr("id", "HistogramSVG" + index)
		.attr("width", (svgWidth + 2 * emFontSize))
		.attr("height", svgHeight)
		.attr("offset", 0.75 * emFontSize)
		//.attr('style', 'position: relative; left: 1em; top: 0em; pointer-events: auto')
		.attr('style', 'position: relative; left: 1em; top: ' + (-histHeight + 0 * emFontSize) + 'px; pointer-events: auto')
		//.style('top', (-histHeight + 0*emFontSize));
		.on("dblclick", function () {
			if (isLocal)
				return;

			d3.select(this)
				.attr("opacity", 0.0)
				.transition()
				.duration(250)
				.attr("opacity", 1.0);

			var strRequest = '[vote] datasetId=' + datasetId + '&flux=' + document.getElementById('flux' + index).value;
			wsConn[index - 1].send(strRequest);
		});

	if (va_count > 1) {
		var imageTag = document.getElementById('imageTag#' + index);

		let line = fitsData.LINE.trim();
		let filter = fitsData.FILTER.trim();

		if (line != "")
			imageTag.innerHTML = plain2chem(line, true);

		if (filter != "")
			imageTag.innerHTML = filter;
	}

	redraw_histogram(index);
}

function Einstein_velocity_addition(v1, v2) {
	var c = 299792.458;//speed of light [km/s]

	return (v1 + v2) / (1 + v1 * v2 / (c * c));
}

function Einstein_relative_velocity(f, f0) {
	var c = 299792.458;//speed of light [km/s]

	var deltaV = 0.0;

	try {
		deltaV = document.getElementById('velocityInput').valueAsNumber;//[km/s]
	}
	catch (e) {
		console.log(e);
		console.log("USER_DELTAV = ", USER_DELTAV);
	}

	//convert redshift z to V
	var value = sessionStorage.getItem("redshift");

	if (value == "z") {
		var tmp = - (1.0 - (1.0 + deltaV) * (1.0 + deltaV)) / (1.0 + (1.0 + deltaV) * (1.0 + deltaV));

		deltaV = tmp * c;
	};

	var fRatio = f / f0;
	var v = (1.0 - fRatio * fRatio) / (1.0 + fRatio * fRatio) * c;

	return Einstein_velocity_addition(v, deltaV);
}

function relativistic_rest_frequency(f) {
	var c = 299792.458;//speed of light [km/s]

	var v = document.getElementById('velocityInput').valueAsNumber;//[km/s]

	var beta = v / c;

	//convert redshift z to V
	var value = sessionStorage.getItem("redshift");

	if (value == "z")
		beta = - (1.0 - (1.0 + v) * (1.0 + v)) / (1.0 + (1.0 + v) * (1.0 + v));

	var tmp = Math.sqrt((1.0 + beta) / (1.0 - beta));

	return f * tmp;
};

function get_spectrum_margin() {
	return 0.1;
}

function setup_3d_view() {
	var svg = d3.select("#FrontSVG");
	var width = parseFloat(svg.attr("width"));
	var height = parseFloat(svg.attr("height"));

	var rect = d3.select("#image_rectangle");
	var rect_width = parseFloat(rect.attr("width"));
	var rect_height = parseFloat(rect.attr("height"));

	var position = (width - rect_width) / 2 - 10 * emFontSize;

	if (va_count > 1 && composite_view)
		position = (width + rect_width) / 2 + 2 * emFontSize;
}

function dragstart() {
	freqdrag = true;
	d3.event.preventDefault = true;

	var offset = d3.mouse(this);
	freq_mouse_start = offset[0];

	var frequency = get_mouse_frequency(offset);

	if (optical_view)
		session_freq_start = frequency;

	if (has_frequency_info) {
		if (frequency > 0.0)
			session_freq_start = frequency;
	}
	else
		if (has_velocity_info)
			session_freq_start = frequency;

	session_freq_end = session_freq_start;//added by Chris on 2018/12/04

	//session_frame_start = get_mouse_frame(offset) ;

	if (has_frequency_info)
		console.log("drag started", freqdrag, freq_mouse_start, (frequency / 1e9).toPrecision(7), "GHz");
	else
		if (has_velocity_info)
			console.log("drag started", freqdrag, freq_mouse_start, (frequency / 1e3).toPrecision(5), "km/s");

	d3.select("#fregion").moveToFront();
}

function dragend() {
	console.log("drag ended");
	freqdrag = false;

	d3.select("#fregion").attr("opacity", 0.0);
	freq_mouse_start = 0;

	d3.select("#fregion").moveToBack();

	d3.select("#freq_bar").attr("opacity", 0.0);

	var freq_start = session_freq_start;
	var freq_end = session_freq_end;
	var tmp = freq_start;

	if (freq_start == freq_end) {
		console.log("ignoring a single-channel region selection!");

		freq_mouse_start = 0;
		freqdrag = false;
		session_freq_start = 0;
		session_freq_end = 0;

		shortcut.remove("f");
		shortcut.remove("Left");
		shortcut.remove("Right");
		shortcut.remove("Enter");
		mol_pos = -1;

		return;
	}

	if (freq_end < freq_start) {
		freq_start = freq_end;
		freq_end = tmp;
	};

	data_band_lo = freq_start;
	data_band_hi = freq_end;

	frame_start = session_frame_start;
	frame_end = session_frame_end;
	//recalculate {data_band_lo,data_band_hi} based on {frame_start,frame_end} 

	//if((freq_start > 0.0) && (freq_end > 0.0))
	// if((frame_start >= 0) && (frame_end >= 0))
	{
		display_hourglass();

		image_count = 0;
		viewport_count = 0;
		spectrum_count = 0;

		for (let index = 1; index <= va_count; index++)
			image_refresh(index);

		display_molecules();
	}

	freq_mouse_start = 0;
	freqdrag = false;
	session_freq_start = 0;
	session_freq_end = 0;

	shortcut.remove("f");
	shortcut.remove("Left");
	shortcut.remove("Right");
	shortcut.remove("Enter");
	mol_pos = -1;
}

function dragmove() {
	var offset = d3.mouse(this);
	var frequency = get_mouse_frequency(offset);

	var freq = d3.select("#frequency");
	var offsetx = parseFloat(freq.attr("x"));

	//console.log("dragmove", frequency.toPrecision(7)) ;

	var x1 = offsetx;
	var x2 = offsetx + parseFloat(freq.attr("width"));
	var x = offset[0];

	if (x < x1) x = x1;
	if (x > x2) x = x2;

	d3.select("#freq_bar").attr("x1", x).attr("x2", x).attr("opacity", 1.0);

	var fregion = d3.select("#fregion");
	var mouseBegin = freq_mouse_start;
	var mouseEnd = offset[0];

	if (mouseEnd < mouseBegin) {
		var mouseTmp = mouseBegin;
		mouseBegin = mouseEnd;

		if (mouseBegin < x1)
			mouseBegin = x1;

		mouseEnd = mouseTmp;
	};

	if (mouseBegin < x1)
		mouseBegin = x1;

	if (mouseEnd > x2)
		mouseEnd = x2;

	var mouseWidth = mouseEnd - mouseBegin;

	fregion.attr("x", mouseBegin).attr("width", mouseWidth).attr("opacity", 0.25);

	if (optical_view)
		session_freq_end = frequency;

	if (has_frequency_info) {
		if (frequency > 0.0)
			session_freq_end = frequency;
	}
	else
		if (has_velocity_info)
			session_freq_end = frequency;

	session_frame_end = get_mouse_frame(offset);

	var freq_start = session_freq_start;
	var freq_end = session_freq_end;
	var tmp = freq_start;

	if (freq_end < freq_start) {
		freq_start = freq_end;
		freq_end = tmp;
	};

	if (has_frequency_info)
		console.log((freq_start / 1e9).toPrecision(7) + " - " + (freq_end / 1e9).toPrecision(7) + " GHz");
	else
		if (has_velocity_info)
			console.log((freq_start / 1e3).toPrecision(5) + " - " + (freq_end / 1e3).toPrecision(5) + " km/s");

	var checkbox = document.getElementById('restcheckbox');

	try {
		if (checkbox.checked)
			frequency = relativistic_rest_frequency(frequency);
	}
	catch (e) {
		if (has_velocity_info)
			d3.select("#jvoText").text((frequency / 1.0e3).toFixed(getVelocityPrecision()) + " km/s");
	};

	if (optical_view)
		d3.select("#jvoText").text(Math.round(frequency));

	if (has_frequency_info) {
		var relvel = Einstein_relative_velocity(frequency, RESTFRQ);

		d3.select("#jvoText").text((frequency / 1.0e9).toPrecision(7) + " " + 'GHz' + ", " + relvel.toFixed(getVelocityPrecision()) + " km/s");
	}
}

function decimalPlaces(num) {
	var match = ('' + num).match(/(?:\.(\d+))?(?:[eE]([+-]?\d+))?$/);
	if (!match) { return 0; }
	return Math.max(
		0,
		// Number of digits right of decimal point.
		(match[1] ? match[1].length : 0)
		// Adjust for scientific notation.
		- (match[2] ? +match[2] : 0));
}

function getFrequencyPrecision() {
	let fitsData = fitsContainer[va_count - 1];

	if (has_velocity_info) {
		return 7;
	}

	if (has_frequency_info) {
		let dF = fitsData.CDELT3 / 1e9;//[GHz]

		console.log("dF = ", dF, "decimal = ", decimalPlaces(dF));

		return decimalPlaces(dF);
	};

	return 7;
}

function getVelocityPrecision() {
	let fitsData = fitsContainer[va_count - 1];

	if (fitsData == null)
		return 1;

	if (has_velocity_info) {
		let dV = fitsData.CDELT3 / 1000;//[km/s]
		//console.log("dV = ", dV, "decimal = ", decimalPlaces(dV)) ;

		let dec = decimalPlaces(dV);
		//return dec ;

		//add an override, some Star Formation FITS files use too many decimal places in CDELT3
		if (dec > 10)
			return 2;
		else
			return dec;
	};

	if (has_frequency_info) {
		/*let dF = fitsData.CDELT3 ;
		let dV = Einstein_relative_velocity(RESTFRQ+dF, RESTFRQ) ;//[km/s]
	
		console.log("dV = ", dV, "decimal = ", decimalPlaces(dV)) ;*/

		return 2;
	};

	return 1;
}

function getMinMaxVel(fitsData) {
	var vMin = null;
	var vMax = null;

	if (has_frequency_info) {
		vMin = Einstein_relative_velocity(data_band_lo, RESTFRQ);
		vMax = Einstein_relative_velocity(data_band_hi, RESTFRQ);

		console.log("setup_axes:", "vMin = ", vMin, "vMax = ", vMax);
	}
	else {
		if (has_velocity_info) {
			vMin = data_band_lo / 1000;//[km/s]
			vMax = data_band_hi / 1000;//[km/s]
		}
		else {
			var c = 299792.458;//speed of light [km/s]

			var vMin = fitsData.CRVAL3 + fitsData.CDELT3 * (1 - fitsData.CRPIX3);
			vMin /= 1000;//[km/s]

			var vMax = fitsData.CRVAL3 + fitsData.CDELT3 * (fitsData.depth - fitsData.CRPIX3);
			vMax /= 1000;//[km/s]
		}
	}

	return { vMin: vMin, vMax: vMax };
}

function composite_data_min_max() {
	data_min = Number.MAX_VALUE;
	data_max = - Number.MAX_VALUE;

	for (let i = 0; i < va_count; i++) {
		let spectrum = null;
		let fitsData = fitsContainer[i];
		let scale = spectrum_scale[i];

		if (intensity_mode == "mean")
			spectrum = fitsData.mean_spectrum;

		if (intensity_mode == "integrated")
			spectrum = fitsData.integrated_spectrum;

		data_min = Math.min(data_min, scale * d3.min(spectrum));
		data_max = Math.max(data_max, scale * d3.max(spectrum));
	}
}

function setup_csv_export() {
	var elem = document.getElementById('exportCSV');

	if (elem == null)
		return;

	console.log("setting up CSV spectrum export");

	elem.onclick = function () {
		console.log("export spectrum to CSV.");

		var c = 299792.458;//speed of light [km/s]

		var deltaV = 0.0;

		try {
			deltaV = document.getElementById('velocityInput').valueAsNumber;//[km/s]
		}
		catch (e) {
			console.log(e);
			console.log("USER_DELTAV = ", USER_DELTAV);
		}

		//convert redshift z to V
		var value = sessionStorage.getItem("redshift");

		if (value == "z") {
			var tmp = - (1.0 - (1.0 + deltaV) * (1.0 + deltaV)) / (1.0 + (1.0 + deltaV) * (1.0 + deltaV));

			deltaV = tmp * c;
		};

		var checkbox = document.getElementById('restcheckbox');
		var rest = false;

		try {
			rest = checkbox.checked;
		} catch (e) {
			console.log(e);
		}

		display_hourglass();

		for (let index = 0; index < va_count; index++) {
			// a CSV websocket request
			var request = {
				type: "csv",
				ra: d3.select("#ra").text().toString(),
				dec: d3.select("#dec").text().toString(),
				intensity: intensity_mode,
				frame_start: data_band_lo,
				frame_end: data_band_hi,
				ref_freq: RESTFRQ,
				deltaV: 1000.0 * deltaV, // [m/s]
				rest: rest,
				seq_id: sent_seq_id,
				timestamp: performance.now(),
			};

			if (wsConn[index].readyState == 1)
				wsConn[index].send(JSON.stringify(request));
		}
	};
}

function setup_axes() {
	let fitsData = fitsContainer[va_count - 1];

	if (fitsData.depth <= 1)
		return;

	try {
		d3.select("#axes").remove();
	}
	catch (e) {
	}

	try {
		d3.select("#foreignCSV").remove();
	}
	catch (e) {
	}

	var svg = d3.select("#BackSVG");
	var width = parseFloat(svg.attr("width"));
	var height = parseFloat(svg.attr("height"));

	svg = svg.append("g").attr("id", "axes");

	var velInfo = getMinMaxVel(fitsData);
	var vMin = velInfo.vMin;
	var vMax = velInfo.vMax;

	if (va_count == 1) {
		var spectrum = null;

		if (intensity_mode == "mean")
			spectrum = fitsData.mean_spectrum;

		if (intensity_mode == "integrated")
			spectrum = fitsData.integrated_spectrum;

		data_min = d3.min(spectrum);
		data_max = d3.max(spectrum);
	}
	else
		composite_data_min_max();

	var dmin = data_min;//d3.min(spectrum) ;
	var dmax = data_max;//d3.max(spectrum) ;

	if (dmin == dmax) {
		if (dmin == 0.0 && dmax == 0.0) {
			dmin = -1.0;
			dmax = 1.0;
		} else {
			if (dmin > 0.0) {
				dmin *= 0.99;
				dmax *= 1.01;
			};

			if (dmax < 0.0) {
				dmax *= 0.99;
				dmin *= 1.01;
			}
		}
	}

	var interval = dmax - dmin;

	var range = get_axes_range(width, height);

	var iR = d3.scaleLinear()
		.range([range.xMin, range.xMax])
		.domain([data_band_lo, data_band_hi]);

	var xR = d3.scaleLinear()
		.range([range.xMin, range.xMax])
		.domain([data_band_lo / 1e9, data_band_hi / 1e9]);

	var vR = d3.scaleLinear()
		.range([range.xMin, range.xMax])
		.domain([vMin, vMax]);

	var yR = d3.scaleLinear()
		.range([range.yMax, range.yMin])
		.domain([dmin - get_spectrum_margin() * interval, dmax + get_spectrum_margin() * interval]);

	var checkbox = document.getElementById('restcheckbox');

	try {
		if (checkbox.checked) {
			xR.domain([relativistic_rest_frequency(data_band_lo) / 1e9, relativistic_rest_frequency(data_band_hi) / 1e9]);

			vR.domain([Einstein_relative_velocity(relativistic_rest_frequency(data_band_lo), RESTFRQ), Einstein_relative_velocity(relativistic_rest_frequency(data_band_hi), RESTFRQ)]);
		}
	}
	catch (e) { };

	var iAxis = d3.axisTop(iR)
		.tickSizeOuter([3])
		.ticks(7);

	var xAxis = d3.axisTop(xR)
		.tickSizeOuter([3])
		.ticks(7);
	/*.tickFormat(function(d) {
		//limit the number of decimal digits shown
		return parseFloat(d.toPrecision(7)) ;
	});*/
	/*.tickFormat(function(d) {var n ;
				 if(fitsData.CDELT3 > 0)
					 n = d * (fitsData.depth-1) + 1 ;
				 else
					 n = (1-d) * (fitsData.depth-1) + 1 ;
				 
				 var freq = fitsData.CRVAL3+fitsData.CDELT3*(n-fitsData.CRPIX3) ;
				 freq /= 1e9 ;//convert from Hz to GHz
				 return freq.toPrecision(6) ;
	});*/

	var vAxis = d3.axisBottom(vR)
		.tickSizeOuter([3]);
	/*.tickFormat(function(d) {var freq = data_band_lo + d * (data_band_hi - data_band_lo) ;
				 var vel = Einstein_relative_velocity(freq, RESTFRQ) ;
				 return vel.toPrecision(6) ;
	});*/

	var yAxis = d3.axisRight(yR)
		.tickSizeOuter([3])
		.tickFormat(function (d) {
			var number;

			if (Math.abs(d) <= 0.001 || Math.abs(d) >= 1000)
				number = d.toExponential();
			else
				number = d;

			if (Math.abs(d) == 0)
				number = d;

			return number;
		});

	if (optical_view) {
		//i-axis label
		var strILabel = "cube frames";

		svg.append("foreignObject")
			.attr("x", (2 * range.xMin + 1.5 * emFontSize))
			.attr("y", (height - 3.5 * emFontSize))
			.attr("width", 20 * emFontSize)
			.attr("height", 2 * emFontSize)
			.append("xhtml:div")
			.attr("id", "frequency_display")
			.style("display", "inline-block")
			.attr("class", "axis-label")
			.html(strILabel);

		// Add the X Axis
		svg.append("g")
			.attr("class", "axis")
			.attr("id", "iaxis")
			.style("fill", "#996699")
			.style("stroke", "#996699")
			//.style("stroke-width", emStrokeWidth)
			.attr("transform", "translate(0," + (height - 1) + ")")
			.call(iAxis);
	}

	if (has_frequency_info) {
		//x-axis label
		var strXLabel = "";

		try {
			if (!checkbox.checked)
				strXLabel = '<I>F<SUB>' + fitsData.SPECSYS.trim() + '</SUB></I> [GHz]';
			else
				strXLabel = '<I>F<SUB>REST</SUB></I> [GHz]';
		}
		catch (e) {
			strXLabel = '<I>F<SUB>' + 'LSRK' + '</SUB></I> [GHz]';
		};

		svg.append("foreignObject")
			.attr("x", (2 * range.xMin + 1.5 * emFontSize))
			.attr("y", (height - 3.5 * emFontSize))
			.attr("width", 20 * emFontSize)
			.attr("height", 2 * emFontSize)
			.append("xhtml:div")
			.attr("id", "frequency_display")
			.style("display", "inline-block")
			.attr("class", "axis-label")
			.html(strXLabel);

		// Add the X Axis
		svg.append("g")
			.attr("class", "axis")
			.attr("id", "xaxis")
			.style("fill", "#996699")
			.style("stroke", "#996699")
			//.style("stroke-width", emStrokeWidth)
			.attr("transform", "translate(0," + (height - 1) + ")")
			.call(xAxis);
	}

	if (!optical_view) {
		//y-axis label
		var yLabel = "Integrated";

		if (intensity_mode == "mean")
			yLabel = "Mean";

		var bunit = '';
		if (fitsData.BUNIT != '') {
			bunit = fitsData.BUNIT.trim();

			if (intensity_mode == "integrated" && has_velocity_info)
				bunit += '•km/s';

			bunit = "[" + bunit + "]";
		}

		svg.append("text")
			.attr("id", "ylabel")
			.attr("x", (-height + 2 * range.xMin + 1.5 * emFontSize)/*-0.75*height*/)
			.attr("y", 1.25 * emFontSize + 0 * range.xMin)
			.attr("font-family", "Inconsolata")
			.attr("font-size", "1.25em")
			.attr("text-anchor", "start")
			.style("fill", "darkgray")
			//.style("opacity", 0.7)
			.attr("stroke", "none")
			.attr("transform", "rotate(-90)")
			.text(yLabel + ' ' + fitsData.BTYPE.trim() + " " + bunit);

		// Add the Y Axis
		svg.append("g")
			.attr("class", "axis")
			.attr("id", "yaxis")
			.style("fill", "#996699")
			.style("stroke", "#996699")
			//.style("stroke-width", emStrokeWidth)
			.attr("transform", "translate(" + (0.75 * range.xMin - 1) + ",0)")
			.call(yAxis);

		// Add a CSV export link
		if (has_velocity_info || has_frequency_info) {
			var front_svg = d3.select("#FrontSVG");
			var width = parseFloat(front_svg.attr("width"));
			var height = parseFloat(front_svg.attr("height"));

			strCSV = '<span id="exportCSV" class="fas fa-file-download" style="display:inline-block; cursor: pointer" title="click to export spectrum to a local file"></span>'

			var colour_style = "csv-dark";
			if (theme == 'bright')
				colour_style = "csv-light";

			let x1 = range.xMax + 0.75 * emFontSize;
			let x2 = (range.xMax + width) / 2.0 - 0.5 * emFontSize;

			front_svg.append("foreignObject")
				.attr("id", "foreignCSV")
				.attr("x", Math.min(x1, x2))
				.attr("y", (height - 2.0 * emFontSize))
				.attr("width", 2 * emFontSize)
				.attr("height", 2 * emFontSize)
				.append("xhtml:div")
				.attr("id", "csv")
				.attr("class", colour_style)
				.attr("pointer-events", "auto")
				.html(strCSV);

			setup_csv_export();

			d3.select("#csv").moveToFront();
		};
	}

	//if(fitsData.CTYPE3 == "FREQ")
	if (vMin != null && vMax != null && !optical_view) {
		var vpos = 0;

		if (!has_frequency_info) {
			vpos = height - 1;

			vAxis = d3.axisTop(vR)
				.tickSizeOuter([3]);
		}

		// Add the V Axis
		svg.append("g")
			.attr("class", "axis")
			.attr("id", "vaxis")
			.style("fill", "#996699")
			.style("stroke", "#996699")
			//.style("stroke-width", emStrokeWidth)
			.attr("transform", "translate(0," + vpos + ")")
			.call(vAxis);

		var strZLabel = "";

		if (fitsData.SPECSYS.trim() != "")
			strZLabel = "<I>V<SUB>" + fitsData.SPECSYS.trim() + "</SUB></I> [km/s]";
		else
			strZLabel = "<I>V<SUB>" + 'LSRK' + "</SUB></I> [km/s]";

		var ypos = 2.0 * emFontSize;

		if (!has_frequency_info)
			ypos = height - 3.5 * emFontSize;

		//z-axis label
		svg.append("foreignObject")
			.attr("x", (2 * range.xMin + 1.5 * emFontSize))
			//.attr("y", (0.02*height+1.5*emFontSize))
			.attr("y", ypos)
			.attr("width", 20 * emFontSize)
			.attr("height", 2 * emFontSize)
			.append("xhtml:div")
			.attr("id", "velocity_display")
			.style("display", "inline-block")
			.attr("class", "axis-label")
			.html(strZLabel);
	}

	{
		svg.append("line")
			.attr("id", "freq_bar")
			.attr("x1", range.xMin)
			.attr("y1", 0)
			.attr("x2", range.xMin)
			.attr("y2", height - 1)
			.style("stroke", "white")
			//.style("stroke-dasharray", ("5, 5, 1, 5"))
			.style("stroke-width", 2 * emStrokeWidth)
			.attr("opacity", 0.0);
	}

	//add the x-axis frequency range selection shadow rectangle
	svg.append("rect")
		.attr("id", "fregion")
		.attr("x", range.xMin)
		.attr("y", 0)
		.attr("width", (range.xMax - range.xMin))
		.attr("height", height - 1)
		.attr("fill", "gray")
		.style("stroke-dasharray", ("1, 5, 1, 1"))
		.attr("opacity", 0.0)
		.moveToBack();

	try {
		d3.select("#axes_selection").remove();
	}
	catch (e) {
	}

	var svg = d3.select("#FrontSVG");

	var group = svg.append("g").attr("id", "axes_selection");

	var patternScale = Math.ceil(((range.xMax - range.xMin) / 200 / 4));

	var patternPath = 'M' + (-1 * patternScale) + ',' + (1 * patternScale) + ' l' + (2 * patternScale) + ',' + (-2 * patternScale) + ' M0,' + (4 * patternScale) + ' l' + (4 * patternScale) + ',' + (-4 * patternScale) + ' M' + (3 * patternScale) + ',' + (5 * patternScale) + ' l' + (2 * patternScale) + ',' + (-2 * patternScale);

	svg.append("pattern")
		.attr("id", "diagonalHatch")
		.attr("patternUnits", "userSpaceOnUse")
		.attr("width", patternScale * 4)
		.attr("height", patternScale * 4)
		.append("path")
		//.attr("d", "M-1,1 l2,-2 M0,4 l4,-4 M3,5 l2,-2")
		.attr("d", patternPath)
		.style("stroke", "gray")
		.style("stroke-width", 1);

	if (has_frequency_info || has_velocity_info || optical_view)
		group.append("rect")
			.attr("id", "frequency")
			.attr("x", range.xMin)
			.attr("y", range.yMax + 1)
			.attr("width", (range.xMax - range.xMin))
			.attr("height", (height - 1 - range.yMax - 1))
			.attr("fill", "url(#diagonalHatch)")
			//.attr("stroke", "white")
			//.style("stroke-dasharray", ("1, 5"))
			.attr("opacity", 0.0)
			.style('cursor', 'pointer')
			.on("mouseleave", function () {
				x_axis_mouseleave();
			})
			.on("mouseenter", function () {
				var offset = d3.mouse(this);
				x_axis_mouseenter(offset);

			})
			.on("mousemove", function () {
				var offset = d3.mouse(this);
				x_axis_mousemove(offset);
			})
			.call(d3.drag()
				.on("start", dragstart)
				.on("drag", dragmove)
				.on("end", dragend));

	//shift/zoom Y-Axis
	group = svg.append("g").attr("id", "y_axis_stretching");

	prev_scale = 1.0;

	group.append("rect")
		.attr("id", "scaling")
		.attr("x", 0)
		.attr("y", range.yMin)
		.attr("width", 2 * 0.75 * range.xMin)
		.attr("height", (range.yMax - range.yMin))
		.attr("fill", "url(#diagonalHatch)")
		.attr("opacity", 0.0)
		.call(d3.drag().on("drag", shifted))
		.call(d3.zoom().scaleExtent([0.1, 10]).on("zoom", scaled))
		.on("mouseleave", function () {
			d3.select(this)
				.style('cursor', '')
				.attr("opacity", 0.0);

			/*d3.select("#yaxis")
			.style("fill", "#996699")
			.style("stroke", "#996699");*/
		})
		.on("mouseenter", function () {
			if (autoscale)
				return;

			if (windowLeft)
				return;

			hide_navigation_bar();

			d3.select(this)
				.style('cursor', 'ns-resize')
				.attr("opacity", 0.5);

			let fillColour = 'white';

			if (theme == 'bright')
				fillColour = 'black';

			d3.select("#yaxis")
				.style("fill", fillColour)
				.style("stroke", fillColour);
		});
}

function x_axis_mouseenter(offset) {
	//send an init_video command via WebSockets
	streaming = true;
	video_stack = new Array(va_count);

	if (viewport_zoom_settings != null) {
		d3.select("#upper").style("stroke", "Gray");
		d3.select("#upperCross").attr("opacity", 0.75);
		d3.select("#upperBeam").attr("opacity", 0.75);
	}

	d3.select("#lower").attr("pointer-events", "none");

	//clear the VideoCanvas
	requestAnimationFrame(function () {
		var c = document.getElementById('VideoCanvas');
		var ctx = c.getContext("2d");

		var width = c.width;
		var height = c.height;

		ctx.clearRect(0, 0, width, height);
		ctx.globalAlpha = 1.0;
	});

	if (va_count == 1) {
		var elem = d3.select("#legend"); elem.attr("opacity", 0);
	}
	else {
		for (let index = 1; index <= va_count; index++) {
			var elem = d3.select("#legend" + index);
			elem.attr("opacity", 0);
		}
	}

	//if (videoFrame == null)
	if (wasm_supported) {
		try {
			//init the HEVC encoder		
			api.hevc_init(va_count);
		} catch (e) { };

		var freq = get_mouse_frequency(offset);

		sent_vid_id++;

		if (composite_view) {
			wsConn[0].send('[init_video] frame=' + freq + '&view=composite' + '&ref_freq=' + RESTFRQ + '&fps=' + vidFPS + '&seq_id=' + sent_vid_id + '&bitrate=' + Math.round(target_bitrate) + '&timestamp=' + performance.now());
			video_stack[0] = [];
		} else for (let index = 0; index < va_count; index++) {
			wsConn[index].send('[init_video] frame=' + freq + '&view=tile' + '&ref_freq=' + RESTFRQ + '&fps=' + vidFPS + '&seq_id=' + sent_vid_id + '&bitrate=' + Math.round(target_bitrate) + '&timestamp=' + performance.now());
			video_stack[index] = [];
		};
	}

	hide_navigation_bar();

	d3.select("#scaling")
		.style('cursor', '')
		.attr("opacity", 0.0);

	d3.select("#yaxis")
		.style("fill", "#996699")
		.style("stroke", "#996699");

	d3.select("#frequency").attr("opacity", 0.5);

	let fillColour = 'white';

	if (theme == 'bright')
		fillColour = 'black';

	d3.select("#xaxis")
		.style("fill", fillColour)
		.style("stroke", fillColour)
		.attr("opacity", 1.0);

	d3.select("#vaxis")
		.style("fill", fillColour)
		.style("stroke", fillColour)
		.attr("opacity", 1.0);

	fillColour = 'white';
	let strokeColour = 'black';

	if (theme == 'bright') {
		fillColour = 'black';
		strokeColour = 'white';
	}

	var svg = d3.select("#BackSVG");
	var width = parseFloat(svg.attr("width"));
	var height = parseFloat(svg.attr("height"));

	jvoText = d3.select("#FrontSVG").append("text")
		.attr("id", "jvoText")
		.attr("x", width / 2)
		.attr("y", height / 2)
		//.attr("font-family", "Arial")		
		.attr("font-family", "Inconsolata")
		.attr("font-weight", "regular")
		.attr("font-size", "5em")
		.attr("text-anchor", "middle")
		.attr("fill", fillColour)
		.attr("stroke", strokeColour)
		.attr("pointer-events", "none")
		.attr("opacity", 1.0);

	shortcut.add("f", set_user_restfrq);
	shortcut.add("Left", x_axis_left);
	shortcut.add("Right", x_axis_right);
	shortcut.add("Enter", go_to_splatalogue);
}

function x_axis_mouseleave() {
	streaming = false;
	video_stack = new Array(va_count);

	//clear the VideoCanvas and reset the Zoom Viewport
	d3.select("#upper").style("stroke", "transparent");
	d3.select("#upperCross").attr("opacity", 0.0);
	d3.select("#upperBeam").attr("opacity", 0.0);
	d3.select("#lower").attr("pointer-events", "auto");

	requestAnimationFrame(function () {
		var c = document.getElementById('VideoCanvas');
		var ctx = c.getContext("2d");

		var width = c.width;
		var height = c.height;

		ctx.clearRect(0, 0, width, height);
		ctx.globalAlpha = 0.0;
	});

	if (va_count == 1) {
		var elem = d3.select("#legend");

		if (displayLegend)
			elem.attr("opacity", 1);
		else
			elem.attr("opacity", 0);
	}
	else {
		for (let index = 1; index <= va_count; index++) {
			var elem = d3.select("#legend" + index);

			if (displayLegend)
				elem.attr("opacity", 1);
			else
				elem.attr("opacity", 0);
		}
	}

	d3.select("#fps").text("");

	//send an end_video command via WebSockets
	if (videoFrame[0] != null) {
		try {
			api.hevc_destroy(va_count);
		} catch (e) { };

		if (composite_view) {
			Module._free(videoFrame[0].ptr);
			Module._free(videoFrame[0].alpha_ptr);
			videoFrame[0].img = null;
			videoFrame[0].ptr = null;
			videoFrame[0].alpha_ptr = null;
			videoFrame[0] = null;

			wsConn[0].send('[end_video]');
			video_stack[0] = [];
		} else for (let index = 0; index < va_count; index++) {
			Module._free(videoFrame[index].ptr);
			Module._free(videoFrame[index].alpha_ptr);
			videoFrame[index].img = null;
			videoFrame[index].ptr = null;
			videoFrame[index].alpha_ptr = null;
			videoFrame[index] = null;

			wsConn[index].send('[end_video]');
			video_stack[index] = [];

			if (va_count > 1)
				refresh_tiles(index + 1);
		}
	}

	shortcut.remove("f");
	shortcut.remove("Left");
	shortcut.remove("Right");
	shortcut.remove("Enter");

	d3.select("#frequency").attr("opacity", 0.0);
	d3.select("#freq_bar").attr("opacity", 0.0);

	d3.select("#xaxis")
		.style("fill", "#996699")
		.style("stroke", "#996699");

	/*d3.select("#yaxis")
	  .style("fill", "#996699")
	  .style("stroke", "#996699");*/

	d3.select("#vaxis")
		.style("fill", "#996699")
		.style("stroke", "#996699");

	//d3.select("#freq_bar").attr("opacity", 0.0);

	d3.select("#jvoText").remove();

	mol_pos = -1;
	var modal = document.getElementById('molecularlist');
	modal.style.display = "none";
}

function x_axis_mousemove(offset) {
	mol_pos = -1;

	x_axis_move(offset);
}

function x_axis_left() {
	var freq = round(get_line_frequency(), 10);

	//console.log("current line frequency = ", freq, "\tmol_pos = ", mol_pos) ;

	//find the next line to the left

	var m = document.getElementsByClassName("molecularp");

	if (m.length <= 0)
		return;

	if (mol_pos < 0) {
		mol_pos = 0;

		for (var i = 0; i < m.length; i++) {
			var tmp = round(parseFloat(m[i].getAttribute("freq")), 10);

			if (tmp >= freq)
				break;

			mol_pos = i;
		};
	}
	else {
		if (mol_pos - 1 >= 0)
			mol_pos--;
	};

	var offset = [parseFloat(m[mol_pos].getAttribute("x")), 0];

	x_axis_move(offset);
};

function x_axis_right() {
	var freq = round(get_line_frequency(), 10);

	//console.log("current line frequency = ", freq, "\tmol_pos = ", mol_pos) ;

	//find the next line to the left

	var m = document.getElementsByClassName("molecularp");

	if (m.length <= 0)
		return;

	if (mol_pos < 0) {
		mol_pos = m.length - 1;

		for (var i = m.length - 1; i >= 0; i--) {
			var tmp = round(parseFloat(m[i].getAttribute("freq")), 10);

			if (tmp <= freq)
				break;

			mol_pos = i;
		};
	}
	else {
		if (mol_pos + 1 <= m.length - 1)
			mol_pos++;
	};


	var offset = [parseFloat(m[mol_pos].getAttribute("x")), 0];

	x_axis_move(offset);
};

function replay_video() {
	if (!video_playback)
		return;

	x_axis_mousemove(video_offset);

	//simulate a mouse advance
	var width = parseFloat(d3.select("#frequency").attr("width"));
	var offsetx = parseFloat(d3.select("#frequency").attr("x"));

	let fps = 30;
	let no_frames = fps * video_period;

	let dx = width / no_frames;
	let dt = 1000.0 / fps;

	var new_video_offset = video_offset[0] + dx;
	if (new_video_offset > offsetx + width)
		new_video_offset = offsetx;

	video_offset[0] = new_video_offset;
	//var dt = video_period / width;

	video_timeout = setTimeout(replay_video, dt);
}

function x_axis_move(offset) {
	clearTimeout(idleVideo);

	let strokeColour = 'white';

	if (theme == 'bright')
		strokeColour = 'black';

	d3.select("#freq_bar")
		.attr("x1", offset[0])
		.attr("x2", offset[0])
		.attr("opacity", 1.0)
		.style("stroke", strokeColour);

	var dx = parseFloat(d3.select("#frequency").attr("width"));
	var offsetx = parseFloat(d3.select("#frequency").attr("x"));

	var band_lo = data_band_lo;
	var band_hi = data_band_hi;

	var freq = band_lo + (offset[0] - offsetx) / dx * (band_hi - band_lo);
	USER_SELFRQ = freq;

	var checkbox = document.getElementById('restcheckbox');

	try {
		if (checkbox.checked) {
			freq = relativistic_rest_frequency(freq);

			USER_SELFRQ = relativistic_rest_frequency(USER_SELFRQ);
		}
	}
	catch (e) {
		if (has_velocity_info)
			d3.select("#jvoText").text((freq / 1.0e3).toFixed(getVelocityPrecision()) + " km/s");

		//return ;//commented out by Chris on 2018/08/03
	};

	console.log("RESTFRQ:", RESTFRQ, "freq:", freq);

	var relvel = Einstein_relative_velocity(freq, RESTFRQ);

	if (optical_view)
		d3.select("#jvoText").text(Math.round(freq));

	if (has_frequency_info)
		d3.select("#jvoText").text((freq / 1.0e9).toPrecision(7) + " " + 'GHz' + ", " + relvel.toFixed(getVelocityPrecision()) + " km/s");

	var modal = document.getElementById('molecularlist');

	if ((offset[0] - offsetx) >= 0.5 * dx) {
		modal.style.right = null;
		modal.style.left = "2.5%";
	}
	else {
		modal.style.right = "2.5%";
		modal.style.left = null;
	};

	let mol_freq = freq;

	if (!freqdrag && wasm_supported) {
		//initially assume 10 frames per second for a video
		//later on use a Kalman Filter to predict the next frame position and request it		
		vidInterval = 1000 / vidFPS;

		now = performance.now();
		elapsed = performance.now() - then;

		var freq = get_mouse_frequency(offset);

		if (elapsed > vidInterval) {
			then = now - (elapsed % vidInterval);

			//for each dataset request a video frame via WebSockets
			sent_vid_id++;

			video_count = 0;

			if (realtime_video) {
				if (composite_view) {
					let strRequest = 'frame=' + freq + '&key=false' + '&view=composite' + '&ref_freq=' + RESTFRQ + '&fps=' + vidFPS + '&seq_id=' + sent_vid_id + '&bitrate=' + Math.round(target_bitrate);
					wsConn[0].send('[video] ' + strRequest + '&timestamp=' + performance.now());
				} else for (let index = 0; index < va_count; index++) {
					let strRequest = 'frame=' + freq + '&key=false' + '&view=tile' + '&ref_freq=' + RESTFRQ + '&fps=' + vidFPS + '&seq_id=' + sent_vid_id + '&bitrate=' + Math.round(target_bitrate);
					wsConn[index].send('[video] ' + strRequest + '&timestamp=' + performance.now());
				}
			};
		};

		if (videoFrame[0] != null)
			idleVideo = setTimeout(videoTimeout, 250, freq);
	};

	zoom_molecules(mol_freq);
}

function zoom_molecules(freq) {
	let fitsData = fitsContainer[va_count - 1];

	if (fitsData == null)
		return;

	if (fitsData.depth <= 1 || molecules.length <= 0)
		return;

	var pos = -1;
	var minDist = 10 * freq;

	var modal = document.getElementById('molecularlist');
	var scroller = zenscroll.createScroller(modal);

	var m = document.getElementsByClassName("molecularp");

	for (var i = 0; i < m.length; i++) {
		m[i].style.color = "inherit";
		m[i].style.fontSize = "100%";
		m[i].style.fontWeight = "normal";

		var tmp = parseFloat(m[i].getAttribute("freq"));
		var dist = Math.abs(freq - tmp);

		if (dist < minDist) {
			minDist = dist;
			pos = i;
		};
	};

	if (mol_pos >= 0)
		pos = mol_pos;

	if (pos > -1) {
		m[pos].style.color = "yellow";
		m[pos].style.fontSize = "130%";
		m[pos].style.fontWeight = "bold";

		pos = Math.max(0, pos - 5);

		// m[pos].scrollIntoView({ block: "start", behavior: "smooth" }); // does not work correctly in Safari
		scroller.to(m[pos], 500); // 'center' or 'to'
	};

	if (m.length > 0 && displayMolecules)
		modal.style.display = "block";
	else
		modal.style.display = "none";
}

function get_mouse_frame(offset) {
	var freq = d3.select("#frequency");
	var dx = parseFloat(freq.attr("width"));
	var offsetx = parseFloat(freq.attr("x"));

	var band_lo = frame_start;
	var band_hi = frame_end;

	var frame = Math.max(frame_start, Math.min(frame_end, Math.round(frame_start + (offset[0] - offsetx) / dx * (frame_end - frame_start))));

	return frame;
}

function get_mouse_frequency(offset) {
	var freq = d3.select("#frequency");
	var dx = parseFloat(freq.attr("width"));
	var offsetx = parseFloat(freq.attr("x"));

	var band_lo = data_band_lo;
	var band_hi = data_band_hi;

	var frequency = Math.max(band_lo, Math.min(band_hi, band_lo + (offset[0] - offsetx) / dx * (band_hi - band_lo)));

	return frequency;
};

function get_line_frequency() {
	var x = parseFloat(d3.select("#freq_bar").attr("x1"));

	var offset = [x, 0];

	var freq = get_mouse_frequency(offset);

	var checkbox = document.getElementById('restcheckbox');

	if (checkbox.checked)
		freq = relativistic_rest_frequency(freq);

	return freq;
};

function go_to_splatalogue() {
	var freq = round(get_line_frequency() / 1e9, 10);//[GHz]
	var offset = 0.01;//10 MHz [GHz]

	var fmin = freq - offset;//[GHz]
	var fmax = freq + offset;//[GHz]

	var url = "http://www.cv.nrao.edu/php/splat/sp_basic.php?el1=el1&el2=el2&ls1=ls1&ls5=ls5&displayRecomb=displayRecomb&displayLovas=displayLovas&displaySLAIM=displaySLAIM&displayJPL=displayJPL&displayCDMS=displayCDMS&displayToyaMA=displayToyaMA&displayOSU=displayOSU&displayLisa=displayLisa&displayRFI=displayRFI&data_version=v3.0&no_atmospheric=no_atmospheric&no_potential=no_potential&no_probable=no_probable&include_only_nrao=include_only_nrao&show_orderedfreq_only=show_orderedfreq_only&chemical_name=&band=any&z=&energy_range_from=&energy_range_to=&energy_range_type=el_cm1&frequency_units=GHz&from=" + fmin + "&to=" + fmax + "&submit=Search";

	var win = window.open(url, '_blank');

	if (win) {
		//Browser has allowed it to be opened
		win.focus();
	} else {
		//Browser has blocked it
		alert('Please allow popups for this website');
	}
};

function getMousePos(e) {
	return { x: e.clientX, y: e.clientY };
}

function get_zoomed_size(width, height, img_width, img_height) {
	var zoomed_size = Math.max(width / 2, height / 2) / golden_ratio;

	if (zoom_shape == "square")
		return zoomed_size;

	if (zoom_shape == "circle")
		return 1.2 * zoomed_size;
}

d3.selection.prototype.moveToFront = function () {
	return this.each(function () {
		this.parentNode.appendChild(this);
	});
};

d3.selection.prototype.moveToBack = function () {
	return this.each(function () {
		var firstChild = this.parentNode.firstChild;
		if (firstChild) {
			this.parentNode.insertBefore(this, firstChild);
		}
	});
};

function setup_viewports() {
	//delete previous instances
	try {
		d3.select("#upper").remove();
		d3.select("#lower").remove();
		d3.select("#upperCross").remove();
		d3.select("#lowerCross").remove();
	}
	catch (e) { };

	var svg = d3.select("#FrontSVG");
	var width = parseFloat(svg.attr("width"));
	var height = parseFloat(svg.attr("height"));

	var imageCanvas = imageContainer[va_count - 1].imageCanvas;
	var scale = get_image_scale(width, height, imageCanvas.width, imageCanvas.height);
	var img_width = scale * imageCanvas.width;
	var img_height = scale * imageCanvas.height;
	var zoomed_size = get_zoomed_size(width, height, img_width, img_height);

	if (zoom_shape == "square") {
		//upper zoom
		svg.append("rect")
			.attr("id", "upper")
			.attr("x", (emStrokeWidth))
			.attr("y", (emStrokeWidth))
			.attr("width", zoomed_size)
			.attr("height", zoomed_size)
			.attr("fill", "transparent")
			.style("stroke", "transparent")
			//.style("stroke-dasharray", ("1, 5, 1"))
			.style("stroke-width", emStrokeWidth / 2)
			.attr("opacity", 1.0)
			.on("mouseover", function () { /*if(windowLeft) return; else swap_viewports();*/ zoom_location = "lower"; var elem = d3.select(this); elem.style("stroke", "transparent"); elem.moveToBack(); d3.select("#lower").moveToFront(); });

		//lower zoom
		svg.append("rect")
			.attr("id", "lower")
			.attr("x", (width - 1 - emStrokeWidth - zoomed_size))
			.attr("y", (height - 1 - emStrokeWidth - zoomed_size))
			.attr("width", zoomed_size)
			.attr("height", zoomed_size)
			.attr("fill", "transparent")
			.style("stroke", "transparent")
			//.style("stroke-dasharray", ("1, 5, 1"))
			.style("stroke-width", emStrokeWidth / 2)
			.attr("opacity", 1.0)
			.on("mouseover", function () { /*if(windowLeft) return; else swap_viewports();*/ zoom_location = "upper"; var elem = d3.select(this); elem.style("stroke", "transparent"); elem.moveToBack(); d3.select("#upper").moveToFront(); });
	};

	if (zoom_shape == "circle") {
		//upper zoom
		svg.append("circle")
			.attr("id", "upper")
			.attr("cx", (emStrokeWidth + zoomed_size / 2))
			.attr("cy", (emStrokeWidth + zoomed_size / 2))
			.attr("r", zoomed_size / 2)
			.attr("fill", "transparent")
			.style("stroke", "transparent")
			//.style("stroke-dasharray", ("1, 5, 1"))
			.style("stroke-width", emStrokeWidth / 2)
			.attr("opacity", 1.0)
			.on("mouseover", function () { /*if(windowLeft) return; else swap_viewports();*/ zoom_location = "lower"; var elem = d3.select(this); elem.style("stroke", "transparent"); elem.moveToBack(); d3.select("#lower").moveToFront(); });

		//lower zoom
		svg.append("circle")
			.attr("id", "lower")
			.attr("cx", (width - 1 - emStrokeWidth - zoomed_size / 2))
			.attr("cy", (height - 1 - emStrokeWidth - zoomed_size / 2))
			.attr("r", zoomed_size / 2)
			.attr("fill", "transparent")
			.style("stroke", "transparent")
			//.style("stroke-dasharray", ("1, 5, 1"))
			.style("stroke-width", emStrokeWidth / 2)
			.attr("opacity", 1.0)
			.on("mouseover", function () { /*if(windowLeft) return; else swap_viewports();*/ zoom_location = "upper"; var elem = d3.select(this); elem.style("stroke", "transparent"); elem.moveToBack(); d3.select("#upper").moveToFront(); });
	};

	var crossSize = 2.0 * emFontSize;

	//upper cross-hair
	svg.append("svg:image")
		.attr("id", "upperCross")
		.attr("x", (emStrokeWidth + (zoomed_size - crossSize) / 2))
		.attr("y", (emStrokeWidth + (zoomed_size - crossSize) / 2))
		//.attr("xlink:href", ROOT_PATH + "plainicon.com-crosshair_white.svg")
		.attr("xlink:href", "https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/plainicon.com-crosshair_white.svg")
		.attr("width", crossSize)
		.attr("height", crossSize)
		.attr("opacity", 0.0);

	//lower cross-hair
	svg.append("svg:image")
		.attr("id", "lowerCross")
		.attr("x", (width - 1 - emStrokeWidth - (zoomed_size + crossSize) / 2))
		.attr("y", (height - 1 - emStrokeWidth - (zoomed_size + crossSize) / 2))
		//.attr("xlink:href", ROOT_PATH + "plainicon.com-crosshair_white.svg")
		.attr("xlink:href", "https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/plainicon.com-crosshair_white.svg")
		.attr("width", crossSize)
		.attr("height", crossSize)
		.attr("opacity", 0.0);
}

function swap_viewports() {
	var canvas = document.getElementById("ZOOMCanvas");
	var ctx = canvas.getContext('2d');
	var width = canvas.width;
	var height = canvas.height;
	ctx.clearRect(0, 0, width, height);

	d3.select("#" + zoom_location + "Cross").attr("opacity", 0.0);
	d3.select("#" + zoom_location + "Beam").attr("opacity", 0.0);

	var elem = d3.select('#' + zoom_location);
	elem.style("stroke", "transparent");
	elem.attr("pointer-events", "none");
	elem.moveToBack();

	if (zoom_location == "upper") {
		d3.select("#lower")
			.attr("pointer-events", "auto")
			.moveToFront();

		zoom_location = "lower";
		return;
	}

	if (zoom_location == "lower") {
		d3.select("#upper")
			.attr("pointer-events", "auto")
			.moveToFront();

		zoom_location = "upper";
		return;
	}
}

function fits_subregion_start() {
	if (freqdrag) return;
	if (optical_view) return;

	clearTimeout(idleMouse);
	moving = true;
	windowLeft = false;

	console.log("fits_subregion_start");

	d3.select("#" + zoom_location).style("stroke", "transparent");
	d3.select("#" + zoom_location + "Cross").attr("opacity", 0.0);
	d3.select("#" + zoom_location + "Beam").attr("opacity", 0.0);

	d3.select("#pixel").text("").attr("opacity", 0.0);
	d3.select("#ra").text("");
	d3.select("#dec").text("");

	{
		var c = document.getElementById("ZOOMCanvas");
		var ctx = c.getContext("2d");
		var width = c.width;
		var height = c.height;
		ctx.clearRect(0, 0, width, height);
	}

	{
		var c = document.getElementById("SpectrumCanvas");
		var ctx = c.getContext("2d");
		var width = c.width;
		var height = c.height;
		ctx.clearRect(0, 0, width, height);
	}

	var offset = d3.mouse(this);
	begin_x = offset[0];
	begin_y = offset[1];
	mousedown = true;
	d3.select("#zoom").attr("opacity", 0.0);
}

function fits_subregion_drag() {
	if (freqdrag) return;
	if (optical_view) return;

	console.log("fits_subregion_drag");

	d3.select("#zoom").attr("opacity", 0.0);

	d3.select(this).style('cursor', 'default');

	d3.select("#pixel").text("").attr("opacity", 0.0);
	d3.select("#ra").text("");
	d3.select("#dec").text("");

	{
		var c = document.getElementById("ZOOMCanvas");
		var ctx = c.getContext("2d");
		var width = c.width;
		var height = c.height;
		ctx.clearRect(0, 0, width, height);
	}

	{
		var c = document.getElementById("SpectrumCanvas");
		var ctx = c.getContext("2d");
		var width = c.width;
		var height = c.height;
		ctx.clearRect(0, 0, width, height);
	}

	if (mousedown) {
		x1 = begin_x;
		y1 = begin_y;

		var offset = d3.mouse(this);

		x2 = offset[0]; y2 = offset[1];

		if (x2 < x1) { x2 = x1; x1 = offset[0]; };
		if (y2 < y1) { y2 = y1; y1 = offset[1]; };

		dx = x2 - x1; dy = y2 - y1;

		d3.select("#region").attr("x", x1).attr("y", y1).attr("width", dx).attr("height", dy).attr("opacity", 1.0);//.5
	}
}

function fits_subregion_end() {
	if (freqdrag) return;
	if (optical_view) return;

	console.log("fits_subregion_end");

	var offset = d3.mouse(this);
	end_x = offset[0];
	end_y = offset[1];

	/*mousedown = false;
	d3.select("#zoom").attr("opacity", 1.0);*/
	d3.select("#region").attr("opacity", 0.0);

	if (end_x == begin_x || end_y == begin_y) {
		console.log("an invalid partial download region");
		return cancel_download();
	}

	if (displayDownloadConfirmation)
		download_confirmation();
	else
		partial_fits_download(d3.select(this).attr("x"), d3.select(this).attr("y"), d3.select(this).attr("width"), d3.select(this).attr("height"));

	/*var onMouseMoveFunc = d3.select(this).on("mousemove");
	d3.select("#image_rectangle").each( onMouseMoveFunc );*/
}

function get_diagonal_image_position(index, width, height) {
	let basex = width / 2;
	let basey = height / 2;
	let t = index / (va_count + 1);
	t = 2 * t - 1;
	let posx = basex + t * 0.5 * width;//0.5 - overlap, 0.6 - no overlap
	let posy = basey + t * height / 4;

	var image_position = { posx: posx, posy: posy };

	return image_position;
}

function get_square_image_position_4(index, width, height) {
	let offset_x = 0, offset_y = 0;

	if (width >= height)
		offset_x = 0.025 * width;
	else
		offset_y = 0.025 * height;

	if (index == 1)
		return { posx: width / 4 - offset_x, posy: height / 4 - offset_y };

	if (index == 2)
		return { posx: width - width / 4 - offset_x, posy: height / 4 + offset_y };

	if (index == 3)
		return { posx: width / 4 + offset_x, posy: height - height / 4 - offset_y };

	if (index == 4)
		return { posx: width - width / 4 + offset_x, posy: height - height / 4 + offset_y };

	return { posx: width / 2, posy: height / 2 };
}

function get_diagonal_image_position_4(index, width, height) {
	//the diagonal line to the left
	if (index < 3) {
		let basex = width / 4;
		let basey = height / 2;

		let t = index / (va_count - 3 + 1);
		t = 2 * t - 1;
		let posx = basex + t * 0.5 * width / 2;
		let posy = basey + t * height / 2;

		return { posx: posx, posy: posy };
	}

	//the diagonal line to the right
	let basex = width - width / 4;
	let basey = height / 2;

	let t = (index - 2) / (va_count - 3 + 1);
	t = 2 * t - 1;
	let posx = basex + t * 0.5 * width / 2;
	let posy = basey + t * height / 2;

	return { posx: posx, posy: posy };
}

function get_image_position_5(index, width, height) {
	if (index < 5)
		return get_square_image_position_4(index, width, height);
	else
		return { posx: width / 2, posy: height / 2 };
}

function get_horizontal_image_position_6(index, width, height) {
	let offset_x = 0, offset_y = 0;

	offset_x = 0.025 * width;
	offset_y = 0.025 * height;

	if (index == 1)
		return { posx: width / 4 - offset_x, posy: height / 4 };

	if (index == 2)
		return { posx: width / 2 - offset_x, posy: height / 4 + offset_y };

	if (index == 3)
		return { posx: 3 * width / 4 - offset_x, posy: height / 4 };

	if (index == 4)
		return { posx: width / 4 + offset_x, posy: height - height / 4 };

	if (index == 5)
		return { posx: width / 2 + offset_x, posy: height - height / 4 - offset_y };

	if (index == 6)
		return { posx: 3 * width / 4 + offset_x, posy: height - height / 4 };

	return { posx: width / 2, posy: height / 2 };
}


function get_vertical_image_position_6(index, width, height) {
	let offset_x = 0, offset_y = 0;

	offset_x = 0.025 * width;
	offset_y = 0.025 * height;

	if (index == 1)
		return { posx: width / 4, posy: height / 4 - offset_y };

	if (index == 2)
		return { posx: width - width / 4, posy: height / 4 + offset_y };

	if (index == 3)
		return { posx: width / 4 + offset_x, posy: height / 2 - offset_y };

	if (index == 4)
		return { posx: width - width / 4 - offset_x, posy: height / 2 + offset_y };

	if (index == 5)
		return { posx: width / 4, posy: height - height / 4 - offset_y };

	if (index == 6)
		return { posx: width - width / 4, posy: height - height / 4 + offset_y };

	return { posx: width / 2, posy: height / 2 };
}

function get_image_position_6(index, width, height) {
	if (width >= height)
		return get_horizontal_image_position_6(index, width, height);
	else
		return get_vertical_image_position_6(index, width, height);
}

function get_diagonal_image_position_7(index, width, height) {
	//the middle diagonal
	if (index <= 3) {
		let basex = width / 2;
		let basey = height / 2;
		let t = index / 4;
		t = 2 * t - 1;
		let posx = basex + t * width / 3;
		let posy = basey - t * height / 2;

		return { posx: posx, posy: posy };
	}

	//the left diagonal
	if (index <= 5) {
		let basex = width / 3.5;
		let basey = height / 3.5;
		let t = (index - 3) / 4;
		t = 2 * t - 1;
		let posx = basex + t * width / 3;
		let posy = basey - t * height / 2;

		return { posx: posx, posy: posy };
	}

	//the right diagonal
	if (index <= 7) {
		let basex = width - width / 8;
		let basey = height - height / 2;
		let t = (index - 5) / 4;
		t = 2 * t - 1;
		let posx = basex + t * width / 3;
		let posy = basey - t * height / 2;

		return { posx: posx, posy: posy };
	}

	return { posx: width / 2, posy: height / 2 };
}

function get_image_position(index, width, height) {
	if (va_count <= 4)
		return get_diagonal_image_position(index, width, height);

	if (va_count == 5)
		return get_image_position_5(index, width, height);

	if (va_count == 6)
		return get_image_position_6(index, width, height);

	if (va_count == 7)
		return get_diagonal_image_position_7(index, width, height);

	return get_diagonal_image_position(index, width, height);
}

function isNumeric(obj) {
	return !isNaN(obj - parseFloat(obj));
}

var isotopes = ["1H", "2H", "3H", "3He", "4He", "6Li", "7Li", "9Be", "10B", "11B", "12C", "13C", "14C", "14N", "15N", "16O", "17O", "18O", "19F", "20Ne", "21Ne", "22Ne", "23Na", "24Mg", "25Mg", "26Mg", "27Al", "28Si", "29Si", "30Si", "31P", "32S", "33S", "34S", "36S", "35Cl", "37Cl", "36Ar", "38Ar", "40Ar", "39K", "40K", "41K", "40Ca", "42Ca", "43Ca", "44Ca", "46Ca", "48Ca", "45Sc", "46Ti", "47Ti", "48Ti", "49Ti", "50Ti", "50V", "51V", "50Cr", "52Cr", "53Cr", "54Cr", "55Mn", "54Fe", "56Fe", "57Fe", "58Fe", "59Co", "58Ni", "60Ni", "61Ni", "62Ni", "64Ni", "63Cu", "65Cu", "64Zn", "66Zn", "67Zn", "68Zn", "70Zn", "69Ga", "71Ga", "70Ge", "72Ge", "73Ge", "74Ge", "76Ge", "75As", "74Se", "76Se", "77Se", "78Se", "80Se", "82Se", "79Br", "81Br", "78Kr", "80Kr", "82Kr", "83Kr", "84Kr", "86Kr", "85Rb", "87Rb", "84Sr", "86Sr", "87Sr", "88Sr", "89Y", "90Zr", "91Zr", "92Zr", "94Zr", "93Nb", "92Mo", "94Mo", "95Mo", "96Mo", "97Mo", "98Mo", "100Mo", "98Tc", "96Ru", "98Ru", "99Ru", "100Ru", "101Ru", "102Ru", "104Ru", "103Rh", "102Pd", "104Pd", "105Pd", "106Pd", "108Pd", "110Pd", "107Ag", "109Ag", "106Cd", "108Cd", "110Cd", "111Cd", "112Cd", "113Cd", "114Cd", "116Cd", "113In", "115In", "112Sn", "114Sn", "115Sn", "116Sn", "117Sn", "118Sn", "119Sn", "120Sn", "122Sn", "124Sn", "121Sb", "123Sb", "120Te", "122Te", "123Te", "124Te", "125Te", "126Te", "128Te", "130Te", "127I", "124Xe", "126Xe", "128Xe", "129Xe", "130Xe", "131Xe", "132Xe", "134Xe", "133Cs", "130Ba", "132Ba", "134Ba", "135Ba", "136Ba", "137Ba", "138Ba", "138La", "139La", "136Ce", "138Ce", "140Ce", "142Ce", "141Pr", "142Nd", "143Nd", "144Nd", "145Nd", "146Nd", "148Nd", "150Nd", "145Pm", "144Sm", "147Sm", "148Sm", "149Sm", "150Sm", "152Sm", "154Sm", "151Eu", "153Eu", "152Gd", "154Gd", "155Gd", "156Gd", "157Gd", "158Gd", "160Gd", "159Tb", "156Dy", "158Dy", "160Dy", "161Dy", "162Dy", "163Dy", "154Dy", "165Ho", "162Er", "164Er", "166Er", "167Er", "168Er", "170Er", "169Tm", "168Yb", "170Yb", "171Yb", "172Yb", "173Yb", "174Yb", "176Yb", "175Lu", "176Lu", "174Hf", "176Hf", "177Hf", "178Hf", "179Hf", "180Hf", "180Ta", "181Ta", "180W", "182W", "183W", "184W", "186W", "185Re", "187Re", "184Os", "186Os", "187Os", "188Os", "189Os", "190Os", "192Os", "191Ir", "193Ir", "190Pt", "192Pt", "194Pt", "195Pt", "196Pt", "198Pt", "197Au", "196Hg", "198Hg", "199Hg", "200Hg", "201Hg", "202Hg", "204Hg", "203Tl", "205Tl", "204Pb", "206Pb", "207Pb", "208Pb", "209Bi", "209Po", "210At", "222Rn", "223Fr", "226Ra", "227Ac", "232Th", "231Pa", "234U", "235U", "238U", "237Np", "244Pu", "243Am", "247Cm", "247Bk", "251Cf", "252Es", "257Fm", "258Md", "259No", "262Lr", "263Rf", "262Db", "266Sg", "264Bh", "269Hs", "268Mt", "272Uun", "272Uuu", "277Uub", "289Uuq", "289Uuh", "292Uuo"];

function chemical_isotopes(line, baseline) {
	var i, j;

	var source = line;
	var dest = '';

	var pos = -1;

	for (i = 0; i < isotopes.length; i++) {
		pos = source.indexOf(isotopes[i]);

		if (pos > -1) {
			console.log("found " + isotopes[i] + " at pos " + pos);

			dest = source.substring(0, pos);

			if (baseline)
				dest += '<SUP style="font-size: smaller; vertical-align:baseline">';
			else
				dest += '<SUP style="font-size: smaller;">';

			var len = isotopes[i].length;

			for (j = 0; j < len; j++) {
				if (isNumeric(isotopes[i].charAt(j))) {
					dest += isotopes[i].charAt(j);
				}
				else {
					dest += "</SUP>" + isotopes[i].substring(j);
					break;
				};
			}

			//append the remaining formula
			dest += source.substring(pos + len);

			//overwrite the source with a revised version
			source = dest;
		};
	};

	return source;
}

function plain2chem(line, baseline) {
	return chemical_isotopes(line, baseline);
}

function add_line_label(index) {
	if (va_count == 1)
		return;

	let fitsData = fitsContainer[index - 1];

	if (fitsData == null)
		return;

	let line = fitsData.LINE.trim();
	let filter = fitsData.FILTER.trim();

	if (line == "")
		//line = "line #" + index ;
		line = datasetId[index - 1];

	console.log("SPECTRAL LINE:", line, "FILTER:", filter);

	var label;

	if (filter == "")
		label = plain2chem(line, false);
	else
		label = filter;

	if (imageContainer[index - 1] == null)
		return;

	let image_bounding_dims = imageContainer[index - 1].image_bounding_dims;

	let c = document.getElementById('HTMLCanvas' + index);
	let width = c.width;
	let height = c.height;

	let scale = get_image_scale(width, height, image_bounding_dims.width, image_bounding_dims.height);

	if (va_count == 2)
		scale = 0.8 * scale;
	else if (va_count == 4)
		scale = 0.6 * scale;
	else if (va_count == 5)
		scale = 0.5 * scale;
	else if (va_count == 6)
		scale = 0.45 * scale;
	else if (va_count == 7)
		scale = 0.45 * scale;
	else
		scale = 2 * scale / va_count;

	let img_width = scale * image_bounding_dims.width;
	let img_height = scale * image_bounding_dims.height;

	let image_position = get_image_position(index, width, height);
	let posx = image_position.posx;
	let posy = image_position.posy;

	var svg = d3.select("#BackSVG");

	let fontColour = 'gray';//white

	if (theme == 'bright')
		fontColour = 'gray';

	if (colourmap == "greyscale" || colourmap == "negative")
		fontColour = "#C4A000";

	svg.append("foreignObject")
		.attr("x", (posx - img_width / 2))
		.attr("y", (posy - img_height / 2 - 1.75 * emFontSize))
		.attr("width", img_width)
		.attr("height", 2 * emFontSize)
		.append("xhtml:div")
		.attr("id", "line_display")
		.html('<p style="text-align: center; font-size:1.5em; font-family: Inconsolata; font-weight: bold; color:' + fontColour + '">' + label + '</p>');
};

function display_composite_legend() {
	var svg = d3.select("#FrontSVG");
	var width = parseFloat(svg.attr("width"));
	var height = parseFloat(svg.attr("height"));

	var group = d3.select("#information");

	//var strLegend = '<div class="container-fluid" style="color: lightgray;">' ;
	var strLegend = '<div class="container-fluid">';

	for (let index = 0; index < va_count; index++) {
		if (index >= 3)
			break;

		let fitsData = fitsContainer[index];
		let line = fitsData.LINE.trim();
		let filter = fitsData.FILTER.trim();

		if (filter != "")
			line = filter;
		else {
			if (line == "")
				line = "line-" + (index + 1);
		}

		//<canvas id="LEG' + line + '" style="width:2em;height:1em;display:inline-block"></canvas>

		//style="height:0.7em;display:inline-block;"
		//scale:&nbsp;
		var strSelect = '<br><div style="font-size:100%; font-family:Inconsolata;float:right;"><label for="scale' + (index + 1) + '" class="control-label"></label><select onchange="javascript:change_spectrum_scale(' + (index + 1) + ')" class="no-form-control" style="max-width:4em;max-height:1.5em;color:black;" id="scale' + (index + 1) + '"><option class="custom-option" value="1">x1</option><option class="custom-option" value="2">x2</option><option class="custom-option" value="5">x5</option><option class="custom-option" value="10">x10</option></select></div>';

		if (index != va_count - 1)
			//strSelect += '<br><hr width="66%">' ;
			strSelect += '<br><br>';

		strLegend += '<div><div id="DIV' + line + '" style="width:5em;height:1em;display:inline-block"><img id="IMG' + line + '" src="" alt="linedash" width="100%" height="100%"></div><span style="font-size:100%; font-family:Inconsolata; color:' + colours[index] + ';">&nbsp;■&nbsp;</span><span style="font-size:100%; font-family:Helvetica; font-weight:bold; nocolor:' + colours[index] + ';">' + plain2chem(line, false) + '</span>&nbsp;' + strSelect + '</div>';
	}

	strLegend += '</div>';

	group.append("g")
		.attr("id", "foreignRGBGroup")
		.style("opacity", 1.0)
		.append("foreignObject")
		.attr("id", "foreignRGB")
		.attr("x", (width - 25 * emFontSize))
		.attr("y", 12.5 * emFontSize)//12.5em 10.5em
		.attr("width", 25 * emFontSize)
		.attr("height", 25 * emFontSize)//10*
		/*.on("mouseenter", function () {
		  d3.select("#foreignRGBGroup").style("opacity", 1.0) ;
			})
			.on("mouseleave", function () {
			d3.select("#foreignRGBGroup").style("opacity", 0.25) ;
			})*/
		.append("xhtml:div")
		.attr("id", "rgbDiv")
		.attr("class", "container-fluid input")
		.style("float", "right")
		.style("padding", "2.5%")
		.append("span")
		.html(strLegend);

	for (let index = 0; index < va_count; index++) {
		let fitsData = fitsContainer[index];
		let line = fitsData.LINE.trim();
		let filter = fitsData.FILTER.trim();

		if (filter != "")
			line = filter;
		else {
			if (line == "")
				line = "line-" + (index + 1);
		}

		let lineCanvas = document.createElement('canvas');
		lineCanvas.style.visibility = "hidden";

		let width = 3 * emFontSize;
		let height = emFontSize;

		lineCanvas.width = width;
		lineCanvas.height = height;

		var ctx = lineCanvas.getContext('2d');

		ctx.save();
		ctx.beginPath();

		ctx.strokeStyle = getStrokeStyle();
		ctx.setLineDash(linedash[index % linedash.length]);
		ctx.lineWidth = 1;
		ctx.strokeWidth = emStrokeWidth;

		ctx.moveTo(0, height / 2);
		ctx.lineTo(width, height / 2);

		ctx.stroke();
		ctx.closePath();
		ctx.restore();

		var src = lineCanvas.toDataURL();

		d3.select("#IMG" + line)
			.attr("src", src);
	}

	try {
		d3.select("#videoControlG").moveToFront();
	} catch (err) { };
}

function setup_image_selection_index(index, topx, topy, img_width, img_height) {
	//delete previous instances	
	try {
		d3.select("#region").remove();
		d3.select("#zoom").remove();
		d3.select("#image_rectangle" + index).remove();
	}
	catch (e) { };

	if (va_count == 1)
		try {
			d3.select("#region").remove();
			d3.select("#zoom").remove();
			d3.select("#image_rectangle").remove();
		}
		catch (e) { };

	var zoom = d3.zoom()
		.scaleExtent([1, 40])
		.on("zoom", tiles_zoom)
		.on("end", tiles_zoomended);

	var drag = d3.drag()
		.on("start", tiles_dragstarted)
		.on("drag", tiles_dragmove)
		.on("end", tiles_dragended);

	now = performance.now();
	then = now;

	//set up the spectrum rendering loop
	function update_spectrum() {

		if (!windowLeft)
			requestAnimationFrame(update_spectrum);

		//spectrum
		try {
			let go_ahead = true;
			let new_seq_id = 0;

			for (let index = 0; index < va_count; index++) {
				let len = spectrum_stack[index].length;

				if (len > 0) {
					let id = spectrum_stack[index][len - 1].id;

					if (id <= last_seq_id)
						go_ahead = false;
					else
						new_seq_id = Math.max(new_seq_id, id);
				}
				else
					go_ahead = false;
			}

			if (go_ahead) {
				last_seq_id = new_seq_id;
				console.log("last_seq_id:", last_seq_id);

				//pop all <va_count> spectrum stacks
				var data = [];

				for (let index = 0; index < va_count; index++) {
					data.push(spectrum_stack[index].pop().spectrum);
					spectrum_stack[index] = [];
				}

				plot_spectrum(data);
				replot_y_axis();

				last_spectrum = data;
			}

		}
		catch (e) {
			console.log(e);
		}
	}

	var svg = d3.select("#FrontSVG");

	var id;

	if (va_count == 1)
		id = "image_rectangle";
	else
		id = "image_rectangle" + index;

	// a fix for Safari
	d3.select(document.body)
		.on('wheel.body', e => { });

	//svg image rectangle for zooming-in
	var rect = svg.append("rect")
		.attr("id", id)
		.attr("class", "image_rectangle")
		.attr("x", topx)
		.attr("y", topy)
		.attr("width", img_width)
		.attr("height", img_height)
		.style('cursor', 'pointer')//'crosshair')//'none' to mask Chrome latency
		.attr("opacity", 0.0)
		.call(drag)
		.call(zoom)
		.on("click", function () {
			if (isLocal) {
				//parse window.location to get the value of filename<index>
				let params = window.location.search.split("&");
				console.log('URL PARAMS:', params);

				let search = 'filename' + index;

				for (let i = 0; i < params.length; i++) {
					if (params[i].indexOf(search) > -1) {
						console.log("found a parameter", params[i]);
						let values = params[i].split("=");

						if (values.length > 1) {
							let val = values[values.length - 1];
							console.log('VALUE:', val);

							window.location = window.location + '&filename=' + val;

							//window.location = window.location + '&filename=' + encodeURIComponent(datasetId[index-1]) ;
						}
					}
				}
			}
			else {
				//parse window.location to get the value of datasetId<index>
				let params = window.location.search.split("&");
				console.log('URL PARAMS:', params);

				let search = 'datasetId' + index;

				for (let i = 0; i < params.length; i++) {
					if (params[i].indexOf(search) > -1) {
						console.log("found a parameter", params[i]);
						let values = params[i].split("=");

						if (values.length > 1) {
							let val = values[values.length - 1];
							console.log('VALUE:', val);

							window.location = window.location + '&datasetId=' + val;

							//window.location = window.location + '&datasetId=' + encodeURIComponent(datasetId[index-1]) ;
						}
					}
				}
			}
		})
		.on("mouseenter", function () {
			hide_navigation_bar();
			console.log("switching active view to", d3.select(this).attr("id"));

			d3.select(this).moveToFront();
			dragging = false;

			windowLeft = false;

			spectrum_stack = new Array(va_count);
			for (let i = 0; i < va_count; i++)
				spectrum_stack[i] = [];

			requestAnimationFrame(update_spectrum);

			var imageElements = document.getElementsByClassName("image_rectangle");

			for (let i = 0; i < imageElements.length; i++) {
				let element = imageElements[i];

				let attr = element.getAttribute("id");
				let idx = attr.substring(15);

				d3.select("#HTMLCanvas" + idx).style('z-index', i + 1);
			}

			var fitsData = fitsContainer[index - 1];

			if (fitsData == null)
				return;

			if (imageContainer[index - 1] == null)
				return;

			let image_bounding_dims = imageContainer[index - 1].image_bounding_dims;

			if (zoom_dims == null) {
				zoom_dims = {
					x1: image_bounding_dims.x1, y1: image_bounding_dims.y1, width: image_bounding_dims.width, height: image_bounding_dims.height, x0: image_bounding_dims.x1 + 0.5 * (image_bounding_dims.width - 1), y0: image_bounding_dims.y1 + 0.5 * (image_bounding_dims.height - 1),
					rx: 0.5,
					ry: 0.5,
					view: null,
					prev_view: null
				};
			}
		})
		.on("mouseleave", function () {
			windowLeft = true;

			spectrum_stack = new Array(va_count);
			for (let i = 0; i < va_count; i++)
				spectrum_stack[i] = [];

			if (xradec != null) {
				let fitsData = fitsContainer[va_count - 1];

				let raText = 'RA N/A';
				let decText = 'DEC N/A';

				if (fitsData.CTYPE1.indexOf("RA") > -1) {
					if (coordsFmt == 'DMS')
						raText = 'α: ' + RadiansPrintDMS(xradec[0]);
					else
						raText = 'α: ' + RadiansPrintHMS(xradec[0]);
				}

				if (fitsData.CTYPE1.indexOf("GLON") > -1)
					raText = 'l: ' + RadiansPrintDMS(xradec[0]);

				if (fitsData.CTYPE1.indexOf("ELON") > -1)
					raText = 'λ: ' + RadiansPrintDMS(xradec[0]);

				if (fitsData.CTYPE2.indexOf("DEC") > -1)
					decText = 'δ: ' + RadiansPrintDMS(xradec[1]);

				if (fitsData.CTYPE2.indexOf("GLAT") > -1)
					decText = 'b: ' + RadiansPrintDMS(xradec[1]);

				if (fitsData.CTYPE2.indexOf("ELAT") > -1)
					decText = 'β: ' + RadiansPrintDMS(xradec[1]);

				d3.select("#ra").text(raText);
				d3.select("#dec").text(decText);
			}
		})
		.on("mousemove", function () {
			//moving = true;
			windowLeft = false;

			if (zoom_dims == null)
				return;

			var offset;

			try {
				offset = d3.mouse(this);
			}
			catch (e) {
				console.log(e);
				return;
			}

			if (isNaN(offset[0]) || isNaN(offset[1]))
				return;

			mouse_position = { x: offset[0], y: offset[1] };

			var fitsData = fitsContainer[index - 1];

			if (fitsData == null)
				return;

			if (imageContainer[index - 1] == null)
				return;

			var imageCanvas = imageContainer[index - 1].imageCanvas;
			var image_bounding_dims = imageContainer[index - 1].image_bounding_dims;

			if (zoom_dims.view != null)
				image_bounding_dims = zoom_dims.view;

			if (dragging) {
				var dx = d3.event.dx;
				var dy = d3.event.dy;

				dx *= image_bounding_dims.width / d3.select(this).attr("width");
				dy *= image_bounding_dims.height / d3.select(this).attr("height");

				image_bounding_dims.x1 = clamp(image_bounding_dims.x1 - dx, 0, imageCanvas.width - 1 - image_bounding_dims.width);
				image_bounding_dims.y1 = clamp(image_bounding_dims.y1 - dy, 0, imageCanvas.height - 1 - image_bounding_dims.height);
			}

			var rx = (mouse_position.x - d3.select(this).attr("x")) / d3.select(this).attr("width");
			var ry = (mouse_position.y - d3.select(this).attr("y")) / d3.select(this).attr("height");
			var x = image_bounding_dims.x1 + rx * (image_bounding_dims.width - 1);
			var y = image_bounding_dims.y1 + ry * (image_bounding_dims.height - 1);

			zoom_dims.x0 = x;
			zoom_dims.y0 = y;
			zoom_dims.rx = rx;
			zoom_dims.ry = ry;
			//zoom_dims.view = { x1: image_bounding_dims.x1, y1: image_bounding_dims.y1, width: image_bounding_dims.width, height: image_bounding_dims.height };

			var orig_x = x * fitsData.width / imageCanvas.width;
			var orig_y = y * fitsData.height / imageCanvas.height;

			try {
				let raText = 'RA N/A';
				let decText = 'DEC N/A';

				if (fitsData.CTYPE1.indexOf("RA") > -1) {
					if (coordsFmt == 'DMS')
						raText = 'α: ' + x2dms(orig_x);
					else
						raText = 'α: ' + x2hms(orig_x);
				}

				if (fitsData.CTYPE1.indexOf("GLON") > -1)
					raText = 'l: ' + x2dms(orig_x);

				if (fitsData.CTYPE1.indexOf("ELON") > -1)
					raText = 'λ: ' + x2dms(orig_x);

				if (fitsData.CTYPE2.indexOf("DEC") > -1)
					decText = 'δ: ' + y2dms(orig_y);

				if (fitsData.CTYPE2.indexOf("GLAT") > -1)
					decText = 'b: ' + y2dms(orig_y);

				if (fitsData.CTYPE2.indexOf("ELAT") > -1)
					decText = 'β: ' + y2dms(orig_y);

				d3.select("#ra").text(raText);
				d3.select("#dec").text(decText);
			}
			catch (err) {
				//use the CD scale matrix
				let radec = CD_matrix(orig_x, fitsData.height - orig_y);

				let raText = 'RA N/A';
				let decText = 'DEC N/A';

				if (fitsData.CTYPE1.indexOf("RA") > -1) {
					if (coordsFmt == 'DMS')
						raText = 'α: ' + RadiansPrintDMS(radec[0]);
					else
						raText = 'α: ' + RadiansPrintHMS(radec[0]);
				}

				if (fitsData.CTYPE1.indexOf("GLON") > -1)
					raText = 'l: ' + RadiansPrintDMS(radec[0]);

				if (fitsData.CTYPE1.indexOf("ELON") > -1)
					raText = 'λ: ' + RadiansPrintDMS(radec[0]);

				if (fitsData.CTYPE2.indexOf("DEC") > -1)
					decText = 'δ: ' + RadiansPrintDMS(radec[1]);

				if (fitsData.CTYPE2.indexOf("GLAT") > -1)
					decText = 'b: ' + RadiansPrintDMS(radec[1]);

				if (fitsData.CTYPE2.indexOf("ELAT") > -1)
					decText = 'β: ' + RadiansPrintDMS(radec[1]);

				d3.select("#ra").text(raText);
				d3.select("#dec").text(decText);
			}
		});

	zoom.scaleTo(rect, zoom_scale);
}

function setup_image_selection() {
	//delete previous instances
	try {
		d3.select("#region").remove();
		d3.select("#zoom").remove();
		d3.select("#image_rectangle").remove();
	}
	catch (e) { };

	var c = document.getElementById("ZOOMCanvas");
	var ctx = c.getContext("2d");

	ctx.mozImageSmoothingEnabled = false;
	ctx.webkitImageSmoothingEnabled = false;
	ctx.msImageSmoothingEnabled = false;
	ctx.imageSmoothingEnabled = false;

	var svg = d3.select("#FrontSVG");
	var width = parseFloat(svg.attr("width"));
	var height = parseFloat(svg.attr("height"));

	var image_bounding_dims = imageContainer[va_count - 1].image_bounding_dims;
	var scale = get_image_scale(width, height, image_bounding_dims.width, image_bounding_dims.height);
	var img_width = scale * image_bounding_dims.width;
	var img_height = scale * image_bounding_dims.height;

	let fillColour = 'white';

	if (theme == 'bright')
		fillColour = 'black';

	//sub-region selection rectangle
	svg.append("rect")
		.attr("id", "region")
		.attr("x", 0)
		.attr("y", 0)
		.attr("width", 0)
		.attr("height", 0)
		.attr("fill", "none")
		.style("stroke", fillColour)
		.style("stroke-dasharray", ("1, 5, 1"))
		.style("stroke-width", emStrokeWidth)
		.attr("opacity", 0.0);

	if (colourmap == "greyscale" || colourmap == "negative")
		fillColour = "#C4A000";

	if (zoom_shape == "square") {
		//zoom selection rectangle
		svg.append("rect")
			.attr("id", "zoom")
			.attr("x", 0)
			.attr("y", 0)
			.attr("width", 0)
			.attr("height", 0)
			.attr("fill", "none")
			.attr("pointer-events", "none")
			.style("stroke", fillColour)
			//.style("stroke-dasharray", ("1, 5, 1"))
			.style("stroke-width", 3 * emStrokeWidth)
			.attr("opacity", 0.0);
	};

	if (zoom_shape == "circle") {
		//zoom selection circle
		svg.append("circle")
			.attr("id", "zoom")
			.attr("cx", 0)
			.attr("cy", 0)
			.attr("r", 0)
			.attr("fill", "none")
			.attr("pointer-events", "none")
			.style("stroke", fillColour)
			//.style("stroke-dasharray", ("1, 5, 1"))
			.style("stroke-width", 3 * emStrokeWidth)
			.attr("opacity", 0.0);
	};

	var zoom_element = d3.select("#zoom");

	var zoom = d3.zoom()
		.scaleExtent([10, 200])//was 200
		.on("zoom", zoomed);

	now = performance.now();
	then = now;

	spec_now = performance.now();
	spec_then = spec_now;

	//set up the spectrum rendering loop
	function update_spectrum() {

		if (!windowLeft)
			requestAnimationFrame(update_spectrum);

		spec_now = performance.now();
		spec_elapsed = spec_now - spec_then;

		//if (spec_elapsed > fpsInterval)
		{
			spec_then = spec_now - (spec_elapsed % fpsInterval);
			//console.log("spectrum interval: " + spec_elapsed.toFixed(3) + " [ms]", "fps = ", Math.round(1000 / spec_elapsed)) ;

			//image
			try {
				let data = image_stack.pop();
				image_stack = [];

				ctx.clearRect(data.px, data.py, data.zoomed_size, data.zoomed_size);

				var imageCanvas;

				if (composite_view)
					imageCanvas = compositeCanvas;
				else
					imageCanvas = imageContainer[va_count - 1].imageCanvas;//if composite_view use compositeCanvas

				if (zoom_shape == "square") {
					ctx.fillStyle = "rgba(0,0,0,0.3)";
					ctx.fillRect(data.px, data.py, data.zoomed_size, data.zoomed_size);

					ctx.drawImage(imageCanvas, data.x - data.clipSize, data.y - data.clipSize, 2 * data.clipSize + 1, 2 * data.clipSize + 1, data.px, data.py, data.zoomed_size, data.zoomed_size);
				}

				if (zoom_shape == "circle") {
					ctx.save();
					ctx.beginPath();
					ctx.arc(data.px + data.zoomed_size / 2, data.py + data.zoomed_size / 2, data.zoomed_size / 2, 0, 2 * Math.PI, true);

					ctx.fillStyle = "rgba(0,0,0,0.3)";
					ctx.fill();

					ctx.closePath();
					ctx.clip();
					ctx.drawImage(imageCanvas, data.x - data.clipSize, data.y - data.clipSize, 2 * data.clipSize + 1, 2 * data.clipSize + 1, data.px, data.py, data.zoomed_size, data.zoomed_size);
					ctx.restore();
				}
			}
			catch (e) {
				//console.log(e) ;
			}

			//spectrum
			try {
				let go_ahead = true;
				let new_seq_id = 0;

				for (let index = 0; index < va_count; index++) {
					let len = spectrum_stack[index].length;

					if (len > 0) {
						let id = spectrum_stack[index][len - 1].id;

						if (id <= last_seq_id)
							go_ahead = false;
						else
							new_seq_id = Math.max(new_seq_id, id);
					}
					else
						go_ahead = false;
				}

				if (go_ahead) {
					last_seq_id = new_seq_id;
					console.log("last_seq_id:", last_seq_id);

					//pop all <va_count> spectrum stacks
					var data = [];

					for (let index = 0; index < va_count; index++) {
						data.push(spectrum_stack[index].pop().spectrum);
						spectrum_stack[index] = [];
					}

					plot_spectrum(data);
					replot_y_axis();

					last_spectrum = data;
				}

			}
			catch (e) {
				console.log(e);
			}

		}
	}

	// a fix for Safari
	d3.select(document.body)
		.on('wheel.body', e => { });

	//svg image rectangle for zooming-in
	var rect = svg.append("rect")
		.attr("id", "image_rectangle")
		.attr("x", (width - img_width) / 2)
		.attr("y", (height - img_height) / 2)
		.attr("width", img_width)
		.attr("height", img_height)
		.style('cursor', 'none')//'crosshair')//'none' to mask Chrome latency
		/*.style("fill", "transparent")
		  .style("stroke", "yellow")
		  .style("stroke-width", emStrokeWidth)
		  .style("stroke-dasharray", ("1, 5, 1"))*/
		.attr("opacity", 0.0)
		.call(d3.drag()
			.on("start", fits_subregion_start)
			.on("drag", fits_subregion_drag)
			.on("end", fits_subregion_end)
		)
		.call(zoom)
		.on("mouseenter", function () {
			hide_navigation_bar();

			try {
				zoom_beam();
			}
			catch (e) {
				console.log('NON-CRITICAL:', e);
			}

			zoom_element.attr("opacity", 1.0);
			d3.select("#pixel").text("").attr("opacity", 0.0);

			document.addEventListener('copy', copy_coordinates);
			shortcut.add("s", function () {
				set_autoscale_range(false);
			});
			shortcut.add("Meta+C", copy_coordinates);

			windowLeft = false;

			spectrum_stack = new Array(va_count);
			for (let i = 0; i < va_count; i++)
				spectrum_stack[i] = [];

			image_stack = [];
			viewport_zoom_settings = null;

			requestAnimationFrame(update_spectrum);

			var offset;

			try {
				offset = d3.mouse(this);
			}
			catch (e) {
				console.log(e);
				return;
			}

			mouse_position = { x: offset[0], y: offset[1] };

			if (!initKalmanFilter)
				initKalman();

			resetKalman();
		})
		.on("mouseleave", function () {
			clearTimeout(idleMouse);

			if (!d3.event.shiftKey)
				windowLeft = true;

			spectrum_stack = new Array(va_count);
			for (let i = 0; i < va_count; i++)
				spectrum_stack[i] = [];

			image_stack = [];

			if (!d3.event.shiftKey) {
				viewport_zoom_settings = null;
				zoom_element.attr("opacity", 0.0);
			};
			requestAnimationFrame(function () {
				ctx.clearRect(0, 0, width, height);
			});

			d3.select("#" + zoom_location).style("stroke", "transparent");
			d3.select("#" + zoom_location + "Cross").attr("opacity", 0.0);
			d3.select("#" + zoom_location + "Beam").attr("opacity", 0.0);

			d3.select("#pixel").text("").attr("opacity", 0.0);

			document.removeEventListener('copy', copy_coordinates);
			shortcut.remove("Meta+C");
			shortcut.remove("s");

			if (d3.event.shiftKey)
				return;

			setup_csv_export();

			if (xradec != null) {
				let fitsData = fitsContainer[va_count - 1];

				let raText = 'RA N/A';
				let decText = 'DEC N/A';

				if (fitsData.CTYPE1.indexOf("RA") > -1) {
					if (coordsFmt == 'DMS')
						raText = 'α: ' + RadiansPrintDMS(xradec[0]);
					else
						raText = 'α: ' + RadiansPrintHMS(xradec[0]);
				}

				if (fitsData.CTYPE1.indexOf("GLON") > -1)
					raText = 'l: ' + RadiansPrintDMS(xradec[0]);

				if (fitsData.CTYPE1.indexOf("ELON") > -1)
					raText = 'λ: ' + RadiansPrintDMS(xradec[0]);

				if (fitsData.CTYPE2.indexOf("DEC") > -1)
					decText = 'δ: ' + RadiansPrintDMS(xradec[1]);

				if (fitsData.CTYPE2.indexOf("GLAT") > -1)
					decText = 'b: ' + RadiansPrintDMS(xradec[1]);

				if (fitsData.CTYPE2.indexOf("ELAT") > -1)
					decText = 'β: ' + RadiansPrintDMS(xradec[1]);

				d3.select("#ra").text(raText);
				d3.select("#dec").text(decText);
			}

			if (mousedown)
				return;

			let fitsData = fitsContainer[va_count - 1];

			if (fitsData != null) {
				if (fitsData.depth > 1) {
					if (va_count == 1) {
						if (intensity_mode == "mean") {
							plot_spectrum([fitsData.mean_spectrum]);
							replot_y_axis();
						}

						if (intensity_mode == "integrated") {
							plot_spectrum([fitsData.integrated_spectrum]);
							replot_y_axis();
						}
					}
					else {
						if (intensity_mode == "mean") {
							plot_spectrum(mean_spectrumContainer);
							replot_y_axis();
						}

						if (intensity_mode == "integrated") {
							plot_spectrum(integrated_spectrumContainer);
							replot_y_axis();
						}
					}
				}
			}
		})
		.on("mousemove", function () {
			if (!autoscale && d3.event.shiftKey) {
				d3.select("#scaling")
					.style('cursor', 'ns-resize')
					.attr("opacity", 0.5);

				let fillColour = 'white';

				if (theme == 'bright')
					fillColour = 'black';

				d3.select("#yaxis")
					.style("fill", fillColour)
					.style("stroke", fillColour);
			}
			else {
				d3.select("#scaling")
					.style('cursor', '')
					.attr("opacity", 0.0);

				d3.select("#yaxis")
					.style("fill", "#996699")
					.style("stroke", "#996699");
			}

			if (freqdrag || d3.event.shiftKey) {
				d3.select(this).style('cursor', 'pointer');
				return;
			}

			d3.select(this).style('cursor', 'none');

			d3.event.preventDefault = true;
			if (!has_image) return;

			let fitsData = fitsContainer[va_count - 1];

			if (fitsData == null)
				return;

			document.getElementById("SpectrumCanvas").getContext('2d').globalAlpha = 1.0;

			moving = true;
			clearTimeout(idleMouse);
			windowLeft = false;

			d3.select("#" + zoom_location).style("stroke", "Gray");
			d3.select("#" + zoom_location + "Cross").attr("opacity", 0.75);
			d3.select("#" + zoom_location + "Beam").attr("opacity", 0.75);

			var offset;

			try {
				offset = d3.mouse(this);
			}
			catch (e) {
				console.log(e);
				return;
			}

			if (isNaN(offset[0]) || isNaN(offset[1]))
				return;

			mouse_position = { x: offset[0], y: offset[1] };
			//updateKalman() ;

			var image_bounding_dims = imageContainer[va_count - 1].image_bounding_dims;
			var imageCanvas = imageContainer[va_count - 1].imageCanvas;
			var x = image_bounding_dims.x1 + (mouse_position.x - d3.select(this).attr("x")) / d3.select(this).attr("width") * (image_bounding_dims.width - 1);
			var y = image_bounding_dims.y1 + (mouse_position.y - d3.select(this).attr("y")) / d3.select(this).attr("height") * (image_bounding_dims.height - 1);

			var orig_x = x * fitsData.width / imageCanvas.width;
			var orig_y = y * fitsData.height / imageCanvas.height;

			try {
				let raText = 'RA N/A';
				let decText = 'DEC N/A';

				if (fitsData.CTYPE1.indexOf("RA") > -1) {
					if (coordsFmt == 'DMS')
						raText = 'α: ' + x2dms(orig_x);
					else
						raText = 'α: ' + x2hms(orig_x);
				}

				if (fitsData.CTYPE1.indexOf("GLON") > -1)
					raText = 'l: ' + x2dms(orig_x);

				if (fitsData.CTYPE1.indexOf("ELON") > -1)
					raText = 'λ: ' + x2dms(orig_x);

				if (fitsData.CTYPE2.indexOf("DEC") > -1)
					decText = 'δ: ' + y2dms(orig_y);

				if (fitsData.CTYPE2.indexOf("GLAT") > -1)
					decText = 'b: ' + y2dms(orig_y);

				if (fitsData.CTYPE2.indexOf("ELAT") > -1)
					decText = 'β: ' + y2dms(orig_y);

				d3.select("#ra").text(raText);
				d3.select("#dec").text(decText);
			}
			catch (err) {
				//use the CD scale matrix
				let radec = CD_matrix(orig_x, fitsData.height - orig_y);

				let raText = 'RA N/A';
				let decText = 'DEC N/A';

				if (fitsData.CTYPE1.indexOf("RA") > -1) {
					if (coordsFmt == 'DMS')
						raText = 'α: ' + RadiansPrintDMS(radec[0]);
					else
						raText = 'α: ' + RadiansPrintHMS(radec[0]);
				}

				if (fitsData.CTYPE1.indexOf("GLON") > -1)
					raText = 'l: ' + RadiansPrintDMS(radec[0]);

				if (fitsData.CTYPE1.indexOf("ELON") > -1)
					raText = 'λ: ' + RadiansPrintDMS(radec[0]);

				if (fitsData.CTYPE2.indexOf("DEC") > -1)
					decText = 'δ: ' + RadiansPrintDMS(radec[1]);

				if (fitsData.CTYPE2.indexOf("GLAT") > -1)
					decText = 'b: ' + RadiansPrintDMS(radec[1]);

				if (fitsData.CTYPE2.indexOf("ELAT") > -1)
					decText = 'β: ' + RadiansPrintDMS(radec[1]);

				d3.select("#ra").text(raText);
				d3.select("#dec").text(decText);
			}

			//for each image
			var pixelText = '';
			var displayPixel = true;
			var PR = ["R:", "G:", "B:"];
			for (let index = 1; index <= va_count; index++) {
				var pixel_range = imageContainer[index - 1].pixel_range;
				var min_pixel = pixel_range.min_pixel;
				var max_pixel = pixel_range.max_pixel;
				var imageFrame = imageContainer[index - 1].imageFrame;

				var alpha_coord = Math.round(y) * imageFrame.w + Math.round(x);
				var pixel_coord = Math.round(y) * imageFrame.stride + Math.round(x);

				var pixel = imageFrame.bytes[pixel_coord];
				var alpha = imageContainer[index - 1].alpha[alpha_coord];
				var pixelVal = get_pixel_flux(pixel, index);
				var prefix = "";

				if (pixel == max_pixel)
					prefix = "≥";

				if (pixel == min_pixel)
					prefix = "≤";

				let bunit = fitsData.BUNIT.trim();
				if (fitsData.depth > 1 && has_velocity_info)
					bunit += '•km/s';

				if (alpha > 0 && !isNaN(pixelVal)) {
					//d3.select("#pixel").text(prefix + pixelVal.toPrecision(3) + " " + bunit).attr("opacity", 1.0) ;
					if (va_count > 1)
						pixelText += PR[index - 1 % PR.length];
					pixelText += prefix + pixelVal.toPrecision(3) + " ";
					displayPixel = displayPixel && true;
				}
				else {
					//d3.select("#pixel").text("").attr("opacity", 0.0) ;
					displayPixel = displayPixel && false;
				}

				if (index == va_count && displayPixel) {
					pixelText += bunit;
					d3.select("#pixel").text(pixelText).attr("opacity", 1.0);
				}
				else
					d3.select("#pixel").text("").attr("opacity", 0.0);
			}

			now = performance.now();
			elapsed = performance.now() - then;

			if (elapsed > fpsInterval + computed + processed && !mousedown)//+ latency, computed, processed
			{
				then = now - (elapsed % fpsInterval);
				//ALMAWS.send('[mouse] t=' + now + ' x=' + offset[0] + ' y=' + offset[1]);

				console.log("refresh interval: " + elapsed.toFixed(3) + " [ms]", "fps = ", Math.round(1000 / elapsed));

				if (!initKalmanFilter)
					initKalman();

				updateKalman();
				//viewport collision detection
				var collision_detected = false;

				if (zoom_shape == "square") {
					var w1 = parseFloat(zoom_element.attr("width"));
					var h1 = parseFloat(zoom_element.attr("height"));

					var tmp = d3.select("#" + zoom_location);
					var x2 = parseFloat(tmp.attr("x"));
					var y2 = parseFloat(tmp.attr("y"));
					var w2 = parseFloat(tmp.attr("width"));
					var h2 = parseFloat(tmp.attr("height"));

					if (zoom_location == "upper")
						if (((offset[0] - w1 / 2) < (x2 + w2)) && (offset[1] - h1 / 2) < (y2 + h2))
							collision_detected = true;

					if (zoom_location == "lower")
						if (((offset[0] + w1 / 2) > x2) && (offset[1] + h1 / 2) > y2)
							collision_detected = true;
				}

				if (zoom_shape == "circle") {
					var r1 = parseFloat(zoom_element.attr("r"));

					var tmp = d3.select("#" + zoom_location);

					var x = parseFloat(tmp.attr("cx"));
					var y = parseFloat(tmp.attr("cy"));
					var r2 = parseFloat(tmp.attr("r"));

					var dx = offset[0] - x;
					var dy = offset[1] - y;
					var rSq = dx * dx + dy * dy;

					if (rSq < (r1 + r2) * (r1 + r2))
						collision_detected = true;
				}

				if (collision_detected/* && zoom_scale > 10*/) {
					//ctx.clearRect(0, 0, c.width, c.height);
					swap_viewports();
				}

				var clipSize = Math.min(image_bounding_dims.width, image_bounding_dims.height) / zoom_scale;
				var sel_width = clipSize * scale;
				var sel_height = clipSize * scale;

				var pred_mouse_x = Math.round(mouse_position.x + last_x.elements[2] * latency);
				var pred_mouse_y = Math.round(mouse_position.y + last_x.elements[3] * latency);
				//var pred_mouse_x = Math.round(mouse_position.x + last_x.elements[0] * latency + 0.5 * last_x.elements[2] * latency * latency) ;
				//var pred_mouse_y = Math.round(mouse_position.y + last_x.elements[1] * latency + 0.5 * last_x.elements[3] * latency * latency) ;

				//console.log("latency = ", latency.toFixed(1), "[ms]", "mx = ", mouse_position.x, "px = ", pred_mouse_x, "my = ", mouse_position.y, "py = ", pred_mouse_y) ;

				var x = image_bounding_dims.x1 + (mouse_position.x - d3.select(this).attr("x")) / d3.select(this).attr("width") * (image_bounding_dims.width - 1);
				var y = image_bounding_dims.y1 + (mouse_position.y - d3.select(this).attr("y")) / d3.select(this).attr("height") * (image_bounding_dims.height - 1);

				var pred_x = image_bounding_dims.x1 + (pred_mouse_x - d3.select(this).attr("x")) / d3.select(this).attr("width") * (image_bounding_dims.width - 1);
				var pred_y = image_bounding_dims.y1 + (pred_mouse_y - d3.select(this).attr("y")) / d3.select(this).attr("height") * (image_bounding_dims.height - 1);

				var fitsX = pred_x * fitsData.width / imageCanvas.width;//x or pred_x
				var fitsY = pred_y * fitsData.height / imageCanvas.height;//y or pred_y
				var fitsSize = clipSize * fitsData.width / imageCanvas.width;

				fitsX = Math.round(fitsX);
				fitsY = Math.round(fitsY);
				fitsSize = Math.round(fitsSize);

				x = Math.round(x);
				y = Math.round(y);
				clipSize = Math.round(clipSize);

				//console.log('active', 'x = ', x, 'y = ', y, 'clipSize = ', clipSize, 'fitsX = ', fitsX, 'fitsY = ', fitsY, 'fitsSize = ', fitsSize) ;
				//let strLog = 'active x = ' + x + ' y = '+ y + ' clipSize = ' + clipSize + ' fitsX = ' + fitsX + ' fitsY = ' + fitsY + ' fitsSize = ' + fitsSize + ' pred_x = ' + pred_x + ' pred_y = ' + pred_y + ' pred_mouse_x = ' + pred_mouse_x + ' pred_mouse_y = ' + pred_mouse_y ;

				//send a spectrum request to the server
				var x1 = Math.round(fitsX - fitsSize);
				var y1 = Math.round((fitsData.height - 1) - (fitsY - fitsSize));
				var x2 = Math.round(fitsX + fitsSize);
				var y2 = Math.round((fitsData.height - 1) - (fitsY + fitsSize));

				if (realtime_spectrum && fitsData.depth > 1 && !optical_view) {
					sent_seq_id++;

					var range = get_axes_range(width, height);
					var dx = range.xMax - range.xMin;

					for (let index = 0; index < va_count; index++) {
						var dataId = datasetId;
						if (va_count > 1)
							dataId = datasetId[index];

						/*let frame_bounds = get_frame_bounds(data_band_lo, data_band_hi, index) ;
						console.log("frame_bounds:", frame_bounds) ;*/

						if (wsConn[index].readyState == 1) {
							let strRequest = 'dx=' + dx + '&x1=' + x1 + '&y1=' + y2 + '&x2=' + x2 + '&y2=' + y1 + '&image=false&beam=' + zoom_shape + '&intensity=' + intensity_mode + '&frame_start=' + data_band_lo + '&frame_end=' + data_band_hi + '&ref_freq=' + RESTFRQ + '&seq_id=' + sent_seq_id;

							wsConn[index].send('[spectrum] ' + strRequest + '&timestamp=' + performance.now());
						}
					}
				}

				if (zoom_shape == "square")
					zoom_element.attr("x", mouse_position.x - sel_width).attr("y", mouse_position.y - sel_height).attr("width", 2 * sel_width).attr("height", 2 * sel_height).attr("opacity", 1.0);

				if (zoom_shape == "circle")
					zoom_element.attr("cx", Math.round(mouse_position.x)).attr("cy", Math.round(mouse_position.y)).attr("r", Math.round(sel_width)).attr("opacity", 1.0);
				//zoom_element.attr("cx", pred_mouse_x).attr("cy", pred_mouse_y).attr("r", Math.round(sel_width)).attr("opacity", 1.0);

				var px, py;

				var zoomed_size = Math.round(get_zoomed_size(width, height, img_width, img_height));

				if (zoom_location == "upper") {
					px = emStrokeWidth;
					py = emStrokeWidth;
				}
				else {
					px = width - 1 - emStrokeWidth - zoomed_size;
					py = height - 1 - emStrokeWidth - zoomed_size;
				}

				zoomed_size = Math.round(zoomed_size);
				px = Math.round(px);
				py = Math.round(py);

				//console.log(x-clipSize, y-clipSize, px, py, 2*clipSize, zoomed_size) ;		

				image_stack.push({ x: x, y: y, clipSize: clipSize, px: px, py: py, zoomed_size: zoomed_size });
				viewport_zoom_settings = { x: x, y: y, clipSize: clipSize, zoomed_size: zoomed_size };
			}

			idleMouse = setTimeout(imageTimeout, 250);//was 250ms + latency
		});

	zoom.scaleTo(rect, zoom_scale);
}

function stripHTML(html) {
	try {
		return $("<p>" + html + "</p>").text(); // jQuery does the heavy lifting
	} catch (_) {
		return html;
	}
}

function screen_molecule(molecule, search) {
	if (search != '') {
		if (molecule.text.indexOf(search) == -1)
			return false;
	}

	var intensity = parseFloat(molecule.cdms);

	if (intensity < displayIntensity)
		return false;

	if (molecule.list == "CDMS")
		return displayCDMS;

	if (molecule.list == "JPL")
		return displayJPL;

	if (molecule.list == "Recomb")
		return displayRecomb;

	if (molecule.list == "SLAIM")
		return displaySLAIM;

	if (molecule.list == "TopModel")
		return displayTopModel;

	if (molecule.list == "OSU")
		return displayOSU;

	if (molecule.list == "Lovas")
		return displayLovas;

	if (molecule.list == "ToyaMA")
		return displayToyaMA;

	return true;
}

function index_molecules() {
	if (molecules.length <= 0)
		return;

	for (var i = 0; i < molecules.length; i++) {
		var molecule = molecules[i];

		// strip any HTML from the name and species (like <sup>, etc.)
		let name = stripHTML(molecule.name.toLowerCase()).trim();
		let species = stripHTML(molecule.species.toLowerCase()).trim();

		molecule.text = name + " " + species;
	}
}

function display_molecules() {
	if (molecules.length <= 0)
		return;

	if (data_band_lo <= 0 || data_band_hi <= 0)
		return;

	var band_lo = data_band_lo;//[Hz]
	var band_hi = data_band_hi;//[Hz]

	// get the search term (if any)
	var searchTerm = stripHTML(document.getElementById('searchInput').value.toLowerCase()).trim();

	var checkbox = document.getElementById('restcheckbox');

	if (checkbox.checked) {
		band_lo = relativistic_rest_frequency(band_lo);
		band_hi = relativistic_rest_frequency(band_hi);
	};

	var svg = d3.select("#BackSVG");
	var width = parseFloat(svg.attr("width"));
	var height = parseFloat(svg.attr("height"));
	var range = get_axes_range(width, height);

	try {
		d3.select("#molecules").remove();
	}
	catch (e) {
	}

	var group = svg.append("g")
		.attr("id", "molecules")
		.attr("opacity", 0.0);

	// filter the molecules
	var mol_list = [];
	for (var i = 0; i < molecules.length; i++) {
		let molecule = molecules[i];

		if (!screen_molecule(molecule, searchTerm))
			continue;

		let f = molecule.frequency * 1e9;

		if ((f >= band_lo) && (f <= band_hi))
			mol_list.push(molecule);
	};

	var num = mol_list.length;

	var fontStyle = Math.round(0.67 * emFontSize) + "px";// Helvetica";
	var strokeStyle = "#FFCC00";

	if (theme == 'bright')
		strokeStyle = 'black';

	/*if(colourmap == "rainbow" || colourmap == "hot")
	strokeStyle = "white";*/

	//and adjust (reduce) the font size if there are too many molecules to display
	if (num > 20)
		fontStyle = Math.max(8, Math.round(0.67 * emFontSize * .25)) + "px";// Helvetica";

	console.log("valid molecules:", num);

	var dx = range.xMax - range.xMin;
	var offsety = height - 1;

	var div_molecules = d3.select("#molecularlist");
	div_molecules.selectAll("*").remove();

	for (var i = 0; i < mol_list.length; i++) {
		let molecule = mol_list[i];
		let f = molecule.frequency * 1e9;

		var x = range.xMin + dx * (f - band_lo) / (band_hi - band_lo);

		var moleculeG = group.append("g")
			.attr("id", "molecule_group")
			.attr("x", x);

		moleculeG.append("line")
			.attr("id", "molecule_line")
			.attr("x1", x)
			.attr("y1", offsety)
			.attr("x2", x)
			.attr("y2", offsety - 1.25 * emFontSize)
			.style("stroke", strokeStyle)
			.style("stroke-width", 1)
			.attr("opacity", 1.0);

		var text;

		if (molecule.species.indexOf("Unidentified") > -1)
			text = "";
		else
			text = molecule.species;

		moleculeG.append("foreignObject")
			.attr("x", (x - 0.5 * emFontSize))
			.attr("y", (offsety - 2.0 * emFontSize))
			.attr("width", (20 * emFontSize))
			.attr("height", (2 * emFontSize))
			.attr("transform", 'rotate(-45 ' + (x - 0.5 * emFontSize) + ',' + (offsety - 2.0 * emFontSize) + ')')
			.attr("opacity", 1.0)
			.append("xhtml:div")
			.style("font-size", fontStyle)
			.style("font-family", "Inconsolata")
			.style("color", strokeStyle)
			.style("display", "inline-block")
			//.append("p")
			.html(text.trim());

		//console.log("spectral line @ x = ",x, (f/1e9).toPrecision(7), text.trim()) ;

		var cdms = '';
		var intensity = molecule.cdms;

		if (intensity != 0.0)
			cmds = ' CDMS/JPL Int. ' + intensity;

		var htmlStr = molecule.name.trim() + ' ' + text.trim() + ' ' + molecule.quantum.trim() + ' <span style="font-size: 80%">(' + molecule.list + ')</span>';

		if (htmlStr.indexOf("Unidentified") > -1)
			htmlStr = molecule.name;

		div_molecules.append("p")
			.attr("class", "molecularp")
			.attr("freq", f)
			.attr("x", x)
			.html((f / 1e9).toPrecision(7) + ' GHz' + ' ' + htmlStr);
	}

	group.moveToBack();

	var elem = d3.select("#molecules");
	if (displayMolecules)
		elem.attr("opacity", 1);
	else
		elem.attr("opacity", 0);
}

function fetch_spectral_lines(datasetId, freq_start, freq_end) {
	var xmlhttp = new XMLHttpRequest();

	//freq_start, freq_end [Hz]
	var url = 'get_molecules?datasetId=' + encodeURIComponent(datasetId) + '&freq_start=' + freq_start + '&freq_end=' + freq_end + '&' + encodeURIComponent(get_js_version());

	xmlhttp.onreadystatechange = function () {
		if (xmlhttp.readyState == 4 && xmlhttp.status == 502) {
			console.log("Connection error, re-fetching molecules after 1 second.");
			setTimeout(function () {
				fetch_spectral_lines(datasetId, freq_start, freq_end);
			}, 1000);
		}

		if (xmlhttp.readyState == 4 && xmlhttp.status == 202) {
			console.log("Server not ready, long-polling molecules again after 500 ms.");
			setTimeout(function () {
				fetch_spectral_lines(datasetId, freq_start, freq_end);
			}, 500);
		}

		if (xmlhttp.readyState == 4 && xmlhttp.status == 200) {
			var response = JSON.parse(xmlhttp.responseText);

			molecules = response.molecules;
			index_molecules();
			console.log("#SPLATALOGUE molecules: ", molecules.length);

			let fitsData = fitsContainer[va_count - 1];

			if (fitsData != null) {
				if (fitsData.depth > 1)
					display_molecules();
			}
		}
	}

	xmlhttp.open("GET", url, true);
	xmlhttp.timeout = 0;
	xmlhttp.send();
};

function fetch_image(datasetId, index, add_timestamp) {
	var xmlhttp = new XMLHttpRequest();

	var url = 'get_image?datasetId=' + encodeURIComponent(datasetId) + '&' + encodeURIComponent(get_js_version());

	if (add_timestamp)
		url += '&timestamp=' + Date.now();

	xmlhttp.onreadystatechange = function () {
		if (xmlhttp.readyState == 4 && xmlhttp.status == 404) {
			hide_hourglass();
			show_not_found();
		}

		if (xmlhttp.readyState == 4 && xmlhttp.status == 415) {
			hide_hourglass();
			show_unsupported_media_type();
		}

		if (xmlhttp.readyState == 4 && xmlhttp.status == 500) {
			hide_hourglass();
			show_critical_error();
		}

		if (xmlhttp.readyState == 4 && xmlhttp.status == 502) {
			console.log("Connection error, re-fetching image after 1 second.");
			setTimeout(function () {
				fetch_image(datasetId, index, true);
			}, 1000);
		}

		if (xmlhttp.readyState == 4 && xmlhttp.status == 202) {
			console.log("Server not ready, long-polling image again after 500ms.");
			setTimeout(function () {
				fetch_image(datasetId, index, false);
			}, 500);
		}

		if (xmlhttp.readyState == 4 && xmlhttp.status == 200) {
			var received_msg = xmlhttp.response;

			if (received_msg instanceof ArrayBuffer) {
				var dv = new DataView(received_msg);
				console.log("FITSImage dataview byte length: ", dv.byteLength);

				var offset = 0;
				var id_length = dv.getUint32(offset, endianness);
				offset += 8;

				var identifier = new Uint8Array(received_msg, offset, id_length);
				identifier = new TextDecoder("utf-8").decode(identifier);
				offset += id_length;

				var width = dv.getUint32(offset, endianness);
				offset += 4;

				var height = dv.getUint32(offset, endianness);
				offset += 4;

				var image_length = dv.getUint32(offset, endianness);
				offset += 8;

				var frame = new Uint8Array(received_msg, offset, image_length);//offset by 8 bytes
				offset += image_length;

				var alpha_length = dv.getUint32(offset, endianness);
				offset += 8;

				var alpha = new Uint8Array(received_msg, offset);
				console.log("image frame identifier (HTTP): ", identifier, "width:", width, "height:", height, "compressed alpha length:", alpha.length);

				var Buffer = require('buffer').Buffer;
				var LZ4 = require('lz4');

				var uncompressed = new Buffer(width * height);
				uncompressedSize = LZ4.decodeBlock(new Buffer(alpha), uncompressed);
				alpha = uncompressed.slice(0, uncompressedSize);

				//the decoder part

				if (identifier == 'VP9') {
					var decoder = new OGVDecoderVideoVP9();
					console.log(decoder);

					decoder.init(function () { console.log("init callback done"); });
					decoder.processFrame(frame, function () {
						process_image(width, height, decoder.frameBuffer.format.displayWidth,
							decoder.frameBuffer.format.displayHeight,
							decoder.frameBuffer.y.bytes,
							decoder.frameBuffer.y.stride,
							alpha,
							index);
					});
				}
			}
		}
	}

	xmlhttp.open("GET", url, true);//"GET" to help with caching
	xmlhttp.responseType = 'arraybuffer';
	xmlhttp.timeout = 0;
	xmlhttp.send();
}

function fetch_spectrum(datasetId, index, add_timestamp) {
	var xmlhttp = new XMLHttpRequest();

	var intensity = 'integrated';//'mean' or 'integrated'
	var url = 'get_spectrum?datasetId=' + encodeURIComponent(datasetId) + '&' + encodeURIComponent(get_js_version());

	if (add_timestamp)
		url += '&timestamp=' + Date.now();

	xmlhttp.onreadystatechange = function () {
		if (xmlhttp.readyState == 4 && xmlhttp.status == 502) {
			console.log("Connection error, re-fetching spectrum after 1 second.");
			setTimeout(function () {
				fetch_spectrum(datasetId, index, true);
			}, 1000);
		}

		if (xmlhttp.readyState == 4 && xmlhttp.status == 202) {
			console.log("Server not ready, long-polling spectrum again after 500ms.");
			setTimeout(function () {
				fetch_spectrum(datasetId, index, false);
			}, 500);
		}

		/*if (xmlhttp.readyState == 4 && xmlhttp.status == 404) {
			spectrum_count++;

			if (spectrum_count == va_count) {
				document.getElementById('welcome').style.display = "none";
				console.log('hiding the loading progress, style =', document.getElementById('welcome').style.display);
			}
		}*/

		if (xmlhttp.readyState == 4 && xmlhttp.status == 200) {
			//document.getElementById('welcome').style.display = "none";
			//console.log('hiding the loading progress, style =', document.getElementById('welcome').style.display);

			//console.log(xmlhttp.responseText) ;

			var fitsData = JSON.parse(xmlhttp.responseText);

			fitsContainer[index - 1] = fitsData;

			optical_view = fitsData.is_optical;

			if ((fitsData.min == 0) && (fitsData.max == 0) && (fitsData.median == 0) && (fitsData.black == 0) && (fitsData.white == 0)) {
				fetch_spectrum(datasetId, index, true);
				return;
			}

			if (!isLocal) {
				//let filesize = fitsData.HEADERSIZE + 4 * fitsData.width*fitsData.height*fitsData.depth*fitsData.polarisation ;
				let filesize = fitsData.filesize;
				let strFileSize = numeral(filesize).format('0.0b');
				d3.select("#FITS").html("full download (" + strFileSize + ")");
			}

			{
				frame_reference_unit(index);

				//rescale CRVAL3 and CDELT3
				fitsData.CRVAL3 *= frame_multiplier;
				fitsData.CDELT3 *= frame_multiplier;

				frame_reference_type(index);

				console.log("has_freq:", has_frequency_info, "has_vel:", has_velocity_info);
			}

			if (index == va_count)
				display_dataset_info();

			if (va_count == 1 || composite_view) {
				try {
					if (index == va_count)
						display_scale_info();
				}
				catch (err) {
				};
			};

			display_histogram(index);

			display_preferences(index);

			display_legend();

			try {
				display_cd_gridlines();
			}
			catch (err) {
				display_gridlines();
			};

			display_beam();

			display_FITS_header(index);

			if (!composite_view)
				add_line_label(index);

			frame_start = 0;
			frame_end = fitsData.depth - 1;

			if (fitsData.depth > 1) {
				//insert a spectrum object to the spectrumContainer at <index-1>
				mean_spectrumContainer[index - 1] = fitsData.mean_spectrum;
				integrated_spectrumContainer[index - 1] = fitsData.integrated_spectrum;

				spectrum_count++;

				if (va_count == 1) {
					setup_axes();

					if (intensity_mode == "mean")
						plot_spectrum([fitsData.mean_spectrum]);

					if (intensity_mode == "integrated")
						plot_spectrum([fitsData.integrated_spectrum]);

					if (molecules.length > 0)
						display_molecules();
				}
				else {
					if (spectrum_count == va_count) {
						console.log("mean spectrumContainer:", mean_spectrumContainer);
						console.log("integrated spectrumContainer:", integrated_spectrumContainer);

						//display an RGB legend in place of REF FRQ			
						display_composite_legend();

						if (composite_view)
							display_rgb_legend();

						setup_axes();

						if (intensity_mode == "mean")
							plot_spectrum(mean_spectrumContainer);

						if (intensity_mode == "integrated")
							plot_spectrum(integrated_spectrumContainer);

						if (molecules.length > 0)
							display_molecules();
					}
				}
			}
			else {
				spectrum_count++;

				if (spectrum_count == va_count) {
					if (composite_view)
						display_rgb_legend();
				}
			}

			if (spectrum_count == va_count) {
				document.getElementById('welcome').style.display = "none";
				console.log('hiding the loading progress, style =', document.getElementById('welcome').style.display);
			}

			//setup_image_selection() ;
			//setup_viewports() ;
		}
	};

	xmlhttp.open("GET", url, true);//"GET" to help with caching
	xmlhttp.timeout = 0;
	xmlhttp.send();
}

function fetch_contours(datasetId) {
	var xmlhttp = new XMLHttpRequest();

	var url = 'get_contours?datasetId=' + encodeURIComponent(datasetId) + '&' + encodeURIComponent(get_js_version());

	xmlhttp.onreadystatechange = function () {
		if (xmlhttp.readyState == 4 && xmlhttp.status == 502) {
			console.log("Connection error, re-fetching contours after 1 second.");
			setTimeout(function () {
				fetch_contours(datasetId);
			}, 1000);
		}

		if (xmlhttp.readyState == 4 && xmlhttp.status == 200) {
			var response = JSON.parse(xmlhttp.responseText);

			console.log(response);
		}
	}

	xmlhttp.open("GET", url, true);
	xmlhttp.timeout = 0;
	xmlhttp.send();
};

function refresh_tiles(index) {
	if (zoom_scale < 1)
		return;

	if (zoom_dims == null)
		return;

	if (zoom_dims.view == null)
		return;

	let image_bounding_dims = zoom_dims.view;

	if (imageContainer[index - 1] == null)
		return;

	let imageCanvas = imageContainer[index - 1].imageCanvas;

	var id;

	if (va_count == 1)
		id = 'HTMLCanvas';
	else
		id = 'HTMLCanvas' + index;

	var c = document.getElementById(id);
	var width = c.width;
	var height = c.height;
	var ctx = c.getContext("2d");

	ctx.mozImageSmoothingEnabled = false;
	ctx.webkitImageSmoothingEnabled = false;
	ctx.msImageSmoothingEnabled = false;
	ctx.imageSmoothingEnabled = false;
	//ctx.globalAlpha=0.9;

	let img_width = 0, img_height = 0;
	try {
		var id;
		if (va_count == 1)
			id = "#image_rectangle";
		else
			id = "#image_rectangle" + index;
		let elem = d3.select(id);
		img_width = elem.attr("width");
		img_height = elem.attr("height");
	} catch (err) {
		return;
	}

	let image_position = get_image_position(index, width, height);
	let posx = image_position.posx;
	let posy = image_position.posy;

	ctx.drawImage(imageCanvas, image_bounding_dims.x1, image_bounding_dims.y1, image_bounding_dims.width, image_bounding_dims.height, Math.round(posx - img_width / 2), Math.round(posy - img_height / 2), Math.round(img_width), Math.round(img_height));

	//add a bounding box			
	if (theme == 'bright')
		ctx.strokeStyle = "white";
	else
		ctx.strokeStyle = "black";

	ctx.lineWidth = 2;

	ctx.rect(Math.round(posx - img_width / 2), Math.round(posy - img_height / 2), Math.round(img_width), Math.round(img_height));
	ctx.stroke();
	//end of a bounding box
}

function tiles_dragstarted() {
	console.log("drag started");

	d3.select(this).style('cursor', 'move');

	dragging = true;
}

function tiles_dragended() {
	console.log("drag ended");

	d3.select(this).style('cursor', 'pointer');

	dragging = false;

	//do not wait, call tileTimeout immediately
	tileTimeout();
}

function tiles_dragmove() {
	console.log("drag move");

	var elem = d3.select(this);
	var onMouseMoveFunc = elem.on("mousemove");
	elem.each(onMouseMoveFunc);

	for (let i = 1; i <= va_count; i++) {
		requestAnimationFrame(function () {
			refresh_tiles(i);
		});
	}
}

function tiles_zoomended() {
	console.log("zoom end");

	//do not wait, call tileTimeout immediately
	tileTimeout();
}

function tiles_zoom() {
	console.log("scale: " + d3.event.transform.k);
	zoom_scale = d3.event.transform.k;
	moving = true;

	if (zoom_dims == null)
		return;

	//rescale the image
	let width = zoom_dims.width;
	let height = zoom_dims.height;

	let new_width = width / zoom_scale;
	let new_height = height / zoom_scale;

	let x0 = zoom_dims.x0;
	let y0 = zoom_dims.y0;
	let rx = zoom_dims.rx;
	let ry = zoom_dims.ry;
	let new_x1 = clamp(x0 - rx * new_width, 0, zoom_dims.width - 1 - new_width);
	let new_y1 = clamp(y0 - ry * new_height, 0, zoom_dims.height - 1 - new_height);

	zoom_dims.view = { x1: new_x1, y1: new_y1, width: new_width, height: new_height };

	console.log("zoom_dims:", zoom_dims);

	for (let i = 1; i <= va_count; i++) {
		requestAnimationFrame(function () {
			refresh_tiles(i);
		});

		//keep zoom scale in sync across all images
		try {
			var id;
			if (va_count == 1)
				id = "#image_rectangle";
			else
				"#image_rectangle" + i;
			var elem = d3.select(id);
			elem.node().__zoom.k = zoom_scale;
		} catch (e) { };
	}

	var tmp = d3.select(this);
	var onMouseMoveFunc = tmp.on("mousemove");
	tmp.each(onMouseMoveFunc);
}

function zoomed() {
	console.log("scale: " + d3.event.transform.k);
	zoom_scale = d3.event.transform.k;

	if (!windowLeft) {
		try {
			zoom_beam();
		}
		catch (e) {
			console.log('NON-CRITICAL:', e);
		}

		var onMouseMoveFunc = d3.select(this).on("mousemove");
		d3.select("#image_rectangle").each(onMouseMoveFunc);
	}
}

function shifted() {
	if (autoscale)
		return;

	if (last_spectrum == null)
		return;

	console.log("y-axis shift:", d3.event.dy);

	var height = parseFloat(d3.select("#scaling").attr("height"));
	var interval = user_data_max - user_data_min;
	var shift = d3.event.dy * interval / height;

	user_data_max += shift;
	user_data_min += shift;

	plot_spectrum(last_spectrum);
	replot_y_axis();
}

function scaled() {
	if (autoscale)
		return;

	if (last_spectrum == null)
		return;

	console.log("y-axis scale:", d3.event.transform.k, "previous:", prev_scale);

	var factor = d3.event.transform.k;

	if (d3.event.transform.k > prev_scale)
		factor = 1.2;

	if (d3.event.transform.k < prev_scale)
		factor = 0.8;

	prev_scale = d3.event.transform.k;

	/*var interval = factor * (tmp_data_max - tmp_data_min) ;
	var middle = (tmp_data_max + tmp_data_min) / 2 ;*/

	var interval = factor * (user_data_max - user_data_min);
	var middle = (user_data_max + user_data_min) / 2;

	user_data_max = middle + interval / 2;
	user_data_min = middle - interval / 2;

	console.log("AFTER:", user_data_min, user_data_max);

	plot_spectrum(last_spectrum);
	replot_y_axis();
}

function videoTimeout(freq) {
	if (!streaming)
		return;

	console.log("video inactive event");

	sent_vid_id++;

	video_count = 0;

	if (composite_view) {
		let strRequest = 'frame=' + freq + '&key=true' + '&view=composite' + '&ref_freq=' + RESTFRQ + '&fps=' + vidFPS + '&seq_id=' + sent_vid_id + '&bitrate=' + Math.round(target_bitrate);
		wsConn[0].send('[video] ' + strRequest + '&timestamp=' + performance.now());
	} else for (let index = 0; index < va_count; index++) {
		let strRequest = 'frame=' + freq + '&key=true' + '&view=tile' + '&ref_freq=' + RESTFRQ + '&fps=' + vidFPS + '&seq_id=' + sent_vid_id + '&bitrate=' + Math.round(target_bitrate);
		wsConn[index].send('[video] ' + strRequest + '&timestamp=' + performance.now());
	}
}

function blink() {
	if (stop_blinking)
		return;

	d3.select("#ping")
		.transition()
		.duration(250)
		.attr("opacity", 0.0)
		.transition()
		.duration(250)
		.attr("opacity", 1.0)
		.on("end", blink);
}

function end_blink() {
	stop_blinking = true;
}

function tileTimeout(force = false) {
	console.log("tile inactive event");

	moving = false;

	if (zoom_dims == null) {
		console.log("tileTimeout: zoom_dims == null");
		return;
	}

	if (zoom_dims.view == null) {
		console.log("tileTimeout: zoom_dims.view == null");
		return;
	}

	let image_bounding_dims = zoom_dims.view;

	if (mousedown || streaming || dragging) {
		console.log("tileTimeout: mousedown:", mousedown, "streaming:", streaming, "dragging:", dragging);
		return;
	}

	//do nothing if the view has not changed
	if (!force && zoom_dims.prev_view != null) {
		let previous = zoom_dims.prev_view;

		if (image_bounding_dims.x1 == previous.x1 && image_bounding_dims.y1 == previous.y1 && image_bounding_dims.width == previous.width && image_bounding_dims.height == previous.height) {
			console.log("tileTimeout: zoom_dims.view == zoom_dims.prev_view");
			console.log("previous:", previous, "view:", image_bounding_dims);
			return;
		}
	}

	zoom_dims.prev_view = { x1: image_bounding_dims.x1, y1: image_bounding_dims.y1, width: image_bounding_dims.width, height: image_bounding_dims.height };

	viewport_count = 0;
	sent_seq_id++;

	var request_images = true;

	var svg = d3.select("#FrontSVG");
	var width = parseFloat(svg.attr("width"));
	var height = parseFloat(svg.attr("height"));

	var range = get_axes_range(width, height);
	var dx = range.xMax - range.xMin;

	for (let index = 0; index < va_count; index++) {
		let img_width = image_bounding_dims.width;
		let img_height = image_bounding_dims.height;

		let view_width = 0, view_height = 0;
		try {
			var id;
			if (va_count == 1)
				id = "#image_rectangle";
			else
				id = "#image_rectangle" + (index + 1);
			let elem = d3.select(id);
			view_width = elem.attr("width");
			view_height = elem.attr("height");
		} catch (err) {
			continue;
		}

		let view_pixels = view_width * view_height;
		let img_pixels = img_width * img_height;

		console.log("viewport: " + view_width + "x" + view_height + " image: " + img_width + "x" + img_height + "view pixels: " + view_pixels + " img pixels: " + img_pixels);

		let image = true;
		let beam = "square";

		if (img_pixels >= view_pixels)
			image = false;

		request_images = request_images && image;

		//convert zoom_dims.view into real FITS coordinates of each dataset
		var fitsData = fitsContainer[index];

		if (fitsData == null)
			continue;

		if (imageContainer[index] == null)
			continue;

		var imageCanvas = imageContainer[index].imageCanvas;

		let x1 = image_bounding_dims.x1 * fitsData.width / imageCanvas.width;
		let y1 = (fitsData.height - 1) - image_bounding_dims.y1 * fitsData.height / imageCanvas.height;
		let x2 = (image_bounding_dims.x1 + image_bounding_dims.width - 1) * fitsData.width / imageCanvas.width;
		let y2 = (fitsData.height - 1) - (image_bounding_dims.y1 + image_bounding_dims.height - 1) * fitsData.height / imageCanvas.height;

		var strRequest = 'dx=' + dx + '&x1=' + clamp(Math.round(x1), 0, fitsData.width - 1) + '&y1=' + clamp(Math.round(y2), 0, fitsData.height - 1) + '&x2=' + clamp(Math.round(x2), 0, fitsData.width - 1) + '&y2=' + clamp(Math.round(y1), 0, fitsData.height - 1) + '&image=' + (image ? 'true' : 'false') + '&beam=' + beam + '&intensity=' + intensity_mode + '&frame_start=' + data_band_lo + '&frame_end=' + data_band_hi + '&ref_freq=' + RESTFRQ + '&seq_id=' + sent_seq_id + '&timestamp=' + performance.now();

		console.log(strRequest);

		wsConn[index].send('[spectrum] ' + strRequest);
	}

	if (request_images) {
		//display_hourglass();
		stop_blinking = false;
		blink();
	}
}

function imageTimeout() {
	console.log("image inactive event");

	if (mousedown || streaming)
		return;

	moving = false;

	//d3.select("#image_rectangle").style('cursor','crosshair');

	console.log("mouse position: ", mouse_position);

	var c = document.getElementById("ZOOMCanvas");
	var ctx = c.getContext("2d");

	ctx.mozImageSmoothingEnabled = false;
	ctx.webkitImageSmoothingEnabled = false;
	ctx.msImageSmoothingEnabled = false;
	ctx.imageSmoothingEnabled = false;

	var svg = d3.select("#FrontSVG");
	var width = parseFloat(svg.attr("width"));
	var height = parseFloat(svg.attr("height"));

	let fitsData = fitsContainer[va_count - 1];
	var image_bounding_dims = imageContainer[va_count - 1].image_bounding_dims;
	var imageCanvas = imageContainer[va_count - 1].imageCanvas;
	var scale = get_image_scale(width, height, image_bounding_dims.width, image_bounding_dims.height);
	var img_width = scale * image_bounding_dims.width;
	var img_height = scale * image_bounding_dims.height;

	var rect_elem = d3.select("#image_rectangle");

	var x = image_bounding_dims.x1 + (mouse_position.x - rect_elem.attr("x")) / rect_elem.attr("width") * (image_bounding_dims.width - 1);
	var y = image_bounding_dims.y1 + (mouse_position.y - rect_elem.attr("y")) / rect_elem.attr("height") * (image_bounding_dims.height - 1);

	console.log("idle", "x", x, "y", y);

	var clipSize = Math.min(image_bounding_dims.width, image_bounding_dims.height) / zoom_scale;
	var sel_width = clipSize * scale;
	var sel_height = clipSize * scale;

	var fitsX = x * fitsData.width / imageCanvas.width;
	var fitsY = y * fitsData.height / imageCanvas.height;
	var fitsSize = clipSize * fitsData.width / imageCanvas.width;

	fitsX = Math.round(fitsX);
	fitsY = Math.round(fitsY);
	fitsSize = Math.round(fitsSize);

	x = Math.round(x);
	y = Math.round(y);
	clipSize = Math.round(clipSize);

	//console.log('idle', 'x = ', x, 'y = ', y, 'clipSize = ', clipSize, 'fitsX = ', fitsX, 'fitsY = ', fitsY, 'fitsSize = ', fitsSize) ;

	//send an image/spectrum request to the server
	var x1 = Math.round(fitsX - fitsSize);
	var y1 = Math.round((fitsData.height - 1) - (fitsY - fitsSize));
	var x2 = Math.round(fitsX + fitsSize);
	var y2 = Math.round((fitsData.height - 1) - (fitsY + fitsSize));

	/*x1 = Math.round(fitsX - fitsSize) ;
	x2 = Math.round(x1 + 2*fitsSize) ;
	y1 = Math.round(fitsY - fitsSize) ;
	y2 = Math.round(y1 + 2*fitsSize) ;*/

	var dimx = x2 - x1 + 1;
	var dimy = y1 - y2 + 1;

	if (dimx != dimy)
		console.log("unequal dimensions:", dimx, dimy, "fitsX =", fitsX, "fitsY =", fitsY, "fitsSize =", fitsSize);

	var zoomed_size = get_zoomed_size(width, height, img_width, img_height);

	console.log("zoomed_size:", zoomed_size);

	if (moving || streaming)
		return;

	compositeViewportCanvas = null;
	compositeViewportImageData = null;
	viewport_count = 0;

	sent_seq_id++;

	// attach a CSV export handler
	if (has_velocity_info || has_frequency_info) {
		console.log("setting up an idle CSV handler");

		_x1 = x1; _x2 = x2; _y1 = y1; _y2 = y2; // global variables

		var elem = document.getElementById('exportCSV');

		if (elem != null) {
			elem.onclick = function () {
				console.log("export viewport to CSV.");

				var c = 299792.458;//speed of light [km/s]

				var deltaV = 0.0;

				try {
					deltaV = document.getElementById('velocityInput').valueAsNumber;//[km/s]
				}
				catch (e) {
					console.log(e);
					console.log("USER_DELTAV = ", USER_DELTAV);
				}

				//convert redshift z to V
				var value = sessionStorage.getItem("redshift");

				if (value == "z") {
					var tmp = - (1.0 - (1.0 + deltaV) * (1.0 + deltaV)) / (1.0 + (1.0 + deltaV) * (1.0 + deltaV));

					deltaV = tmp * c;
				};

				var checkbox = document.getElementById('restcheckbox');
				var rest = false;

				try {
					rest = checkbox.checked;
				} catch (e) {
					console.log(e);
				}

				display_hourglass();

				for (let index = 0; index < va_count; index++) {
					// a CSV websocket request
					var request = {
						type: "csv",
						ra: d3.select("#ra").text().toString(),
						dec: d3.select("#dec").text().toString(),
						x1: _x1,
						y1: _y2, // reversed Y-axis
						x2: _x2,
						y2: _y1, // reversed Y-axis
						beam: zoom_shape,
						intensity: intensity_mode,
						frame_start: data_band_lo,
						frame_end: data_band_hi,
						ref_freq: RESTFRQ,
						deltaV: 1000.0 * deltaV, // [m/s]
						rest: rest,
						seq_id: sent_seq_id,
						timestamp: performance.now(),
					};

					if (wsConn[index].readyState == 1)
						wsConn[index].send(JSON.stringify(request));
				}
			};
		}
	}

	var range = get_axes_range(width, height);
	var dx = range.xMax - range.xMin;

	for (let index = 0; index < va_count; index++) {

		var strRequest = 'dx=' + dx + '&x1=' + x1 + '&y1=' + y2 + '&x2=' + x2 + '&y2=' + y1 + '&image=true&beam=' + zoom_shape + '&intensity=' + intensity_mode + '&frame_start=' + data_band_lo + '&frame_end=' + data_band_hi + '&ref_freq=' + RESTFRQ + '&seq_id=' + sent_seq_id + '&timestamp=' + performance.now();

		wsConn[index].send('[spectrum] ' + strRequest);
	}

	if (moving || streaming)
		return;

	var zoom_element = d3.select("#zoom");

	//in the meantime repaint the selection element and the zoom canvas
	if (zoom_shape == "square")
		zoom_element.attr("x", mouse_position.x - sel_width).attr("y", mouse_position.y - sel_height).attr("width", 2 * sel_width).attr("height", 2 * sel_height).attr("opacity", 1.0);

	if (zoom_shape == "circle")
		zoom_element.attr("cx", Math.round(mouse_position.x)).attr("cy", Math.round(mouse_position.y)).attr("r", Math.round(sel_width)).attr("opacity", 1.0);

	var px, py;

	if (zoom_location == "upper") {
		px = emStrokeWidth;
		py = emStrokeWidth;
	}
	else {
		px = width - 1 - emStrokeWidth - zoomed_size;
		py = height - 1 - emStrokeWidth - zoomed_size;
	}

	zoomed_size = Math.round(zoomed_size);
	px = Math.round(px);
	py = Math.round(py);

	//ctx.clearRect(px, py, zoomed_size, zoomed_size);

	var imageCanvas;

	if (composite_view)
		imageCanvas = compositeCanvas;
	else
		imageCanvas = imageContainer[va_count - 1].imageCanvas;//if composite_view use compositeCanvas

	if (zoom_shape == "square") {
		//ctx.fillStyle = "rgba(0,0,0,0.3)";
		//ctx.fillRect(px, py, zoomed_size, zoomed_size);	
		ctx.drawImage(imageCanvas, x - clipSize, y - clipSize, 2 * clipSize + 1, 2 * clipSize + 1, px, py, zoomed_size, zoomed_size);
	}

	if (zoom_shape == "circle") {
		ctx.save();
		ctx.beginPath();
		ctx.arc(px + zoomed_size / 2, py + zoomed_size / 2, zoomed_size / 2, 0, 2 * Math.PI, true);
		//ctx.fillStyle = "rgba(0,0,0,0.3)";
		//ctx.fill() ;
		ctx.closePath();
		ctx.clip();
		ctx.drawImage(imageCanvas, x - clipSize, y - clipSize, 2 * clipSize + 1, 2 * clipSize + 1, px, py, zoomed_size, zoomed_size);
		ctx.restore();
	}
}

function resetKalman() {
	last_x = $V([mouse_position.x, mouse_position.y, 0, 0]);
	//last_x = $V([0, 0, 0, 0]);
	last_velX = 0;
	last_velY = 0;
	last_xPos = mouse_position.x;
	last_yPos = mouse_position.y;
	last_t = performance.now();
}

function initKalman() {
	A = $M([
		[1, 0, 1, 0],
		[0, 1, 0, 1],
		[0, 0, 1, 0],
		[0, 0, 0, 1]
	]);

	B = $M([
		[1, 0, 0, 0],
		[0, 1, 0, 0],
		[0, 0, 1, 0],
		[0, 0, 0, 1]
	]);

	H = $M([
		[1, 0, 1, 0],
		[0, 1, 0, 1],
		[0, 0, 0, 0],
		[0, 0, 0, 0]
	]);

	Q = $M([
		[0, 0, 0, 0],
		[0, 0, 0, 0],
		[0, 0, 0.1, 0],
		[0, 0, 0, 0.1]
	]);

	R = $M([
		[100, 0, 0, 0],
		[0, 100, 0, 0],
		[0, 0, 1000, 0],
		[0, 0, 0, 1000]
	]);

	resetKalman();

	last_P = $M([
		[0, 0, 0, 0],
		[0, 0, 0, 0],
		[0, 0, 0, 0],
		[0, 0, 0, 0]
	]);

	initKalmanFilter = true;
}

function updateKalman() {
	cur_xPos = mouse_position.x;
	cur_yPos = mouse_position.y;

	var now = performance.now();
	var dt = now - last_t;

	if (dt == 0)
		return;

	last_t = now;

	//update A and H to take into account dt
	A.elements[0][2] = dt;
	A.elements[1][3] = dt;

	/*** KALMAN FILTER CODE ***/
	var velX = (cur_xPos - last_x.elements[0]) / dt;
	var velY = (cur_yPos - last_x.elements[1]) / dt;

	/*var velX = (cur_xPos - last_xPos)/dt;
	var velY = (cur_yPos - last_yPos)/dt;
	var accX = (velX - last_velX)/dt;
	var accY = (velY - last_velY)/dt;

	last_xPos = cur_xPos ;
	last_yPos = cur_yPos ;
	last_velX = velX ;
	last_velY = velY ;*/

	var measurement = $V([cur_xPos, cur_yPos, velX, velY]);
	//var measurement = $V([velX, velY, accX, accY]);
	var control = $V([0, 0, 0, 0]); // TODO - adjust

	// prediction
	var x = (A.multiply(last_x)).add(B.multiply(control));
	var P = ((A.multiply(last_P)).multiply(A.transpose())).add(Q);

	// correction
	var S = ((H.multiply(P)).multiply(H.transpose())).add(R);
	var K = (P.multiply(H.transpose())).multiply(S.inverse());
	var y = measurement.subtract(H.multiply(x));

	var cur_x = x.add(K.multiply(y));
	var cur_P = ((Matrix.I(4)).subtract(K.multiply(H))).multiply(P);

	last_x = cur_x;
	last_P = cur_P;
	/**************************/

	//return ;

	//console.log("mouse_position: x=", mouse_position.x, "y=", mouse_position.y) ;
	//console.log("K:", K) ;
	//console.log("Kalman Filter X=", cur_x.elements[0], "Y=",cur_x.elements[1], "Vx=", cur_x.elements[2], "Vy=",cur_x.elements[3]) ;
	//console.log("Kalman Filter Vx=", cur_x.elements[0], "Vy=",cur_x.elements[1], "Ax=", cur_x.elements[2], "Ay=",cur_x.elements[3]) ;

	return;

	/*mouse_position.x = cur_x.elements[0];
	mouse_position.y = cur_x.elements[1];

	return;

	//extrapolation
	var predX = last_x;
	var count = 5;//how many frames ahead

	for (var i = 0; i < count; i++)
		predX = (A.multiply(predX)).add(B.multiply(control));

	console.log("extrapolation: x=", predX.elements[0], "y=", predX.elements[1]);

	mouse_position.x = predX.elements[0];
	mouse_position.y = predX.elements[1];*/
}

function change_noise_sensitivity(refresh, index) {
	noise_sensitivity = document.getElementById('sensitivity' + index).value;

	var multiplier = get_noise_sensitivity(noise_sensitivity);
	document.getElementById('sensitivityInput' + index).innerHTML = get_noise_sensitivity_string(noise_sensitivity, 2);

	var c = document.getElementById("HistogramCanvas" + index);
	var svg = d3.select("#HistogramSVG" + index);
	var width = c.width;
	var height = c.height;

	var flux_elem = d3.select("#flux_path" + index);

	var black, white, median;

	try {
		black = parseFloat(flux_elem.attr("black"));
	}
	catch (e) {
	};

	try {
		white = parseFloat(flux_elem.attr("white"));
	}
	catch (e) {
	};

	try {
		median = parseFloat(flux_elem.attr("median"));
	}
	catch (e) {
	};

	var path = get_flux_path(width, height, document.getElementById('flux' + index).value, black, white, median, multiplier, index);

	flux_elem.attr("d", path);

	if (refresh) {
		display_hourglass();


		if (!composite_view) {
			image_count = va_count - 1;

			image_refresh(index, false);
		}
		else {
			image_count = 0;

			for (let i = 1; i <= va_count; i++)
				image_refresh(i, false);
		}
	}
}

function partial_fits_download(offsetx, offsety, width, height) {
	mousedown = false;
	d3.select("#region").attr("opacity", 0.0);

	let fitsData = fitsContainer[va_count - 1];
	var image_bounding_dims = imageContainer[va_count - 1].image_bounding_dims;
	var imageCanvas = imageContainer[va_count - 1].imageCanvas;

	var x1 = image_bounding_dims.x1 + (begin_x - offsetx) / width * (image_bounding_dims.width - 1);
	var y1 = image_bounding_dims.y1 + (begin_y - offsety) / height * (image_bounding_dims.height - 1);

	var orig_x1 = x1 * fitsData.width / imageCanvas.width;
	var orig_y1 = fitsData.height - y1 * fitsData.height / imageCanvas.height;

	var x2 = image_bounding_dims.x1 + (end_x - offsetx) / width * (image_bounding_dims.width - 1);
	var y2 = image_bounding_dims.y1 + (end_y - offsety) / height * (image_bounding_dims.height - 1);

	var orig_x2 = x2 * fitsData.width / imageCanvas.width;
	var orig_y2 = fitsData.height - y2 * fitsData.height / imageCanvas.height;

	var url = "get_fits?";

	if (va_count == 1)
		url += "datasetId=" + encodeURIComponent(datasetId) + "&";
	else {
		for (let index = 1; index <= va_count; index++)
			url += "datasetId" + index + "=" + encodeURIComponent(datasetId[index - 1]) + "&";
	}

	url += "x1=" + Math.round(orig_x1) + "&y1=" + Math.round(orig_y2) + "&x2=" + Math.round(orig_x2) + "&y2=" + Math.round(orig_y1) + "&frame_start=" + data_band_lo + "&frame_end=" + data_band_hi + "&ref_freq=" + RESTFRQ;

	//console.log(url) ;
	//window.location.assign(url);
	window.open(url, '_blank');
}

function ok_download() {
	$('#downloadConfirmation').modal('hide');
	d3.select("#downloadConfirmation").remove();

	partial_fits_download(d3.select("#image_rectangle").attr("x"), d3.select("#image_rectangle").attr("y"), d3.select("#image_rectangle").attr("width"), d3.select("#image_rectangle").attr("height"));
};

function cancel_download() {
	mousedown = false;
	d3.select("#region").attr("opacity", 0.0);

	$('#downloadConfirmation').modal('hide');
	d3.select("#downloadConfirmation").remove();
};

function show_download_confirmation() {
	var modal = document.getElementById('downloadConfirmation');
	var span = document.getElementById('downloadconfirmationclose');

	// When the user clicks on <span> (x), close the modal
	span.onclick = function () {
		$('#downloadConfirmation').modal('hide');
		d3.select("#downloadConfirmation").remove();

		mousedown = false;
		d3.select("#region").attr("opacity", 0.0);
	}
	// When the user clicks a mouse, close it
	window.onclick = function (event) {
		if (event.target == modal) {
			$('#downloadConfirmation').modal('hide');
			d3.select("#downloadConfirmation").remove();

			mousedown = false;
			d3.select("#region").attr("opacity", 0.0);
		}
	}
}

function show_scaling_help() {
	var modal = document.getElementById('scalingHelp');
	var span = document.getElementById('scalingHeaderClose');

	// When the user clicks on <span> (x), close the modal
	span.onclick = function () {
		$('#scalingHelp').modal('hide');
		d3.select("#scalingHelp").remove();
	}
	// When the user moves a mouse, close it
	window.onmousemove = function (event) {
		if (event.target == modal) {
			$('#scalingHelp').modal('hide');
			d3.select("#scalingHelp").remove();
		}
	}
}

function show_fits_header() {
	$("#fitsHeader").modal("show");

	var modal = document.getElementById('fitsHeader');
	var span = document.getElementById('fitsHeaderClose');

	// When the user clicks on <span> (x), close the modal
	span.onclick = function () {
		$("#fitsHeader").modal("hide");
	}
	// When the user clicks anywhere outside of the modal, close it
	window.onclick = function (event) {
		if (event.target == modal) {
			$("#fitsHeader").modal("hide");
		}
	}
}

function change_intensity_threshold(refresh) {
	displayIntensity = parseFloat(document.getElementById('intensity').value);

	var htmlStr = displayIntensity.toFixed(1);

	if (displayIntensity == 0)
		htmlStr = "-" + htmlStr;

	d3.select("#intVal").html(htmlStr);

	if (refresh) {
		console.log("displayIntensity:", displayIntensity);
		localStorage.setItem("displayIntensity", displayIntensity);
		display_molecules();
	}
}

function hide_navigation_bar() {
	console.log("hide_navigation_bar");
	document.getElementById('menu').style.display = "none";
	d3.select("#menu_activation_area").attr("opacity", 0.1);//was 0.7
}

function display_menu() {
	var div = d3.select("body").append("div")
		.attr("id", "menu")
		.attr("class", "menu");
	//.on("mouseleave", hide_navigation_bar);

	var nav = div.append("nav").attr("class", "navbar navbar-inverse navbar-fixed-top");

	var main = nav.append("div")
		.attr("class", "container-fluid");

	var header = main.append("div")
		.attr("class", "navbar-header");

	header.append("a")
		.attr("href", "https://www.nao.ac.jp/")
		.append("img")
		.attr("class", "navbar-left")
		.attr("src", "https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/logo_naoj_nothing_s.png")
		.attr("alt", "NAOJ")
		.attr("max-height", "100%")
		.attr("height", 50);//2.5*emFontSize);//50

	var mainUL = main.append("ul")
		.attr("class", "nav navbar-nav");

	//FITS
	var fitsMenu = mainUL.append("li")
		.attr("class", "dropdown");

	fitsMenu.append("a")
		.attr("class", "dropdown-toggle")
		.attr("data-toggle", "dropdown")
		.style('cursor', 'pointer')
		.html('FITS <span class="fas fa-folder-open"></span> <span class="caret"></span>');

	var fitsDropdown = fitsMenu.append("ul")
		.attr("class", "dropdown-menu");

	fitsDropdown.append("li")
		.append("a")
		.style('cursor', 'pointer')
		.on("click", show_fits_header)
		.html('display header');

	if (!isLocal && va_count == 1 && (window.location.search.indexOf('ALMA') > 0 || window.location.search.indexOf('ALMB'))) {
		var url = "";

		if (datasetId.localeCompare("ALMA01000000") < 0)
			url = "http://jvo.nao.ac.jp/portal/alma/sv.do?action=download.fits&dataId=";
		else
			url = "http://jvo.nao.ac.jp/portal/alma/archive.do?action=download.fits&dataId=";

		fitsDropdown.append("li")
			.append("a")
			.attr("id", "FITS")
			.attr("href", url + datasetId + '_00_00_00')
			.html('full FITS download <span class="fas fa-save"></span>');
	}
	else {
		fitsDropdown.append("li")
			.append("a")
			.attr("id", "FITS")
			.attr("disabled", "disabled")
			.style("display", "none")
			.style("font-style", "italic")
			.style('cursor', 'not-allowed')
			.html('full FITS download (disabled)');
	}

	//IMAGE
	var imageMenu = mainUL.append("li")
		.attr("class", "dropdown");

	imageMenu.append("a")
		.attr("class", "dropdown-toggle")
		.attr("data-toggle", "dropdown")
		.style('cursor', 'pointer')
		.html('Image <span class="caret"></span>');

	var imageDropdown = imageMenu.append("ul")
		.attr("id", "imageDropdown")
		.attr("class", "dropdown-menu");
	//.style("background-color", "rgba(0,0,0,0.4)");    

	//PREFERENCES
	var prefMenu = mainUL.append("li")
		.attr("class", "dropdown");

	prefMenu.append("a")
		.attr("class", "dropdown-toggle")
		.attr("data-toggle", "dropdown")
		.style('cursor', 'pointer')
		.html('Preferences <span class="caret"></span>');

	var prefDropdown = prefMenu.append("ul")
		.attr("id", "prefDropdown")
		.attr("class", "dropdown-menu");

	//SPLATALOGUE
	if (!optical_view) {
		var splatMenu = mainUL.append("li")
			.attr("id", "splatMenu")
			.attr("class", "dropdown");

		splatMenu.append("a")
			.attr("class", "dropdown-toggle")
			.attr("data-toggle", "dropdown")
			.style('cursor', 'pointer')
			.html('Splatalogue <span class="caret"></span>');

		var splatDropdown = splatMenu.append("ul")
			.attr("class", "dropdown-menu");

		splatDropdown.append("li")
			.append("a")
			.html('<label>intensity cutoff < <span id="intVal">' + displayIntensity.toFixed(1) + '</span> <input id="intensity" class="slider" type="range" min="-10" max="0" step="0.1" value="' + displayIntensity + '" onmousemove="javascript:change_intensity_threshold(false);" onchange="javascript:change_intensity_threshold(true);"/></label>');

		splatDropdown.append("li")
			.html('<label>&nbsp;search for:&nbsp;<input class="form-control search" type="text" id="searchInput" value="" placeholder="water, H2O, CH3, etc." onmouseenter="javascript:this.focus();"/></label>');

		//add onblur
		var m = document.getElementById('searchInput');
		m.onblur = display_molecules;
		m.onmouseleave = display_molecules;
		m.onkeyup = function (e) {
			var event = e || window.event;
			var charCode = event.which || event.keyCode;

			if (charCode == '13') {
				// Enter pressed
				clearTimeout(idleSearch);
				display_molecules();
				return false;
			} else {
				clearTimeout(idleSearch);
				idleSearch = setTimeout(display_molecules, 250);
			}
		}

		var htmlStr;

		htmlStr = displayCDMS ? '<span class="fas fa-check-square"></span> CDMS' : '<span class="far fa-square"></span> CDMS';
		splatDropdown.append("li")
			.append("a")
			.style('cursor', 'pointer')
			.on("click", function () {
				displayCDMS = !displayCDMS;
				localStorage_write_boolean("displayCDMS", displayCDMS);
				var htmlStr = displayCDMS ? '<span class="fas fa-check-square"></span> CDMS' : '<span class="far fa-square"></span> CDMS';
				d3.select(this).html(htmlStr);
				display_molecules();
			})
			.html(htmlStr);

		htmlStr = displayJPL ? '<span class="fas fa-check-square"></span> JPL' : '<span class="far fa-square"></span> JPL';
		splatDropdown.append("li")
			.append("a")
			.style('cursor', 'pointer')
			.on("click", function () {
				displayJPL = !displayJPL;
				localStorage_write_boolean("displayJPL", displayJPL);
				var htmlStr = displayJPL ? '<span class="fas fa-check-square"></span> JPL' : '<span class="far fa-square"></span> JPL';
				d3.select(this).html(htmlStr);
				display_molecules();
			})
			.html(htmlStr);

		htmlStr = displayLovas ? '<span class="fas fa-check-square"></span> Lovas' : '<span class="far fa-square"></span> Lovas';
		splatDropdown.append("li")
			.append("a")
			.style('cursor', 'pointer')
			.on("click", function () {
				displayLovas = !displayLovas;
				localStorage_write_boolean("displayLovas", displayLovas);
				var htmlStr = displayLovas ? '<span class="fas fa-check-square"></span> Lovas' : '<span class="far fa-square"></span> Lovas';
				d3.select(this).html(htmlStr);
				display_molecules();
			})
			.html(htmlStr);

		htmlStr = displayOSU ? '<span class="fas fa-check-square"></span> OSU' : '<span class="far fa-square"></span> OSU';
		splatDropdown.append("li")
			.append("a")
			.style('cursor', 'pointer')
			.on("click", function () {
				displayOSU = !displayOSU;
				localStorage_write_boolean("displayOSU", displayOSU);
				var htmlStr = displayOSU ? '<span class="fas fa-check-square"></span> OSU' : '<span class="far fa-square"></span> OSU';
				d3.select(this).html(htmlStr);
				display_molecules();
			})
			.html(htmlStr);

		htmlStr = displayRecomb ? '<span class="fas fa-check-square"></span> Recomb' : '<span class="far fa-square"></span> Recomb';
		splatDropdown.append("li")
			.append("a")
			.style('cursor', 'pointer')
			.on("click", function () {
				displayRecomb = !displayRecomb;
				localStorage_write_boolean("displayRecomb", displayRecomb);
				var htmlStr = displayRecomb ? '<span class="fas fa-check-square"></span> Recomb' : '<span class="far fa-square"></span> Recomb';
				d3.select(this).html(htmlStr);
				display_molecules();
			})
			.html(htmlStr);

		htmlStr = displaySLAIM ? '<span class="fas fa-check-square"></span> SLAIM' : '<span class="far fa-square"></span> SLAIM';
		splatDropdown.append("li")
			.append("a")
			.style('cursor', 'pointer')
			.on("click", function () {
				displaySLAIM = !displaySLAIM;
				localStorage_write_boolean("displaySLAIM", displaySLAIM);
				var htmlStr = displaySLAIM ? '<span class="fas fa-check-square"></span> SLAIM' : '<span class="far fa-square"></span> SLAIM';
				d3.select(this).html(htmlStr);
				display_molecules();
			})
			.html(htmlStr);

		htmlStr = displayTopModel ? '<span class="fas fa-check-square"></span> TopModel' : '<span class="far fa-square"></span> TopModel';
		splatDropdown.append("li")
			.append("a")
			.style('cursor', 'pointer')
			.on("click", function () {
				displayTopModel = !displayTopModel;
				localStorage_write_boolean("displayTopModel", displayTopModel);
				var htmlStr = displayTopModel ? '<span class="fas fa-check-square"></span> TopModel' : '<span class="far fa-square"></span> TopModel';
				d3.select(this).html(htmlStr);
				display_molecules();
			})
			.html(htmlStr);

		htmlStr = displayToyaMA ? '<span class="fas fa-check-square"></span> ToyaMA' : '<span class="far fa-square"></span> ToyaMA';
		splatDropdown.append("li")
			.append("a")
			.style('cursor', 'pointer')
			.on("click", function () {
				displayToyaMA = !displayToyaMA;
				localStorage_write_boolean("displayToyaMA", displayToyaMA);
				var htmlStr = displayToyaMA ? '<span class="fas fa-check-square"></span> ToyaMA' : '<span class="far fa-square"></span> ToyaMA';
				d3.select(this).html(htmlStr);
				display_molecules();
			})
			.html(htmlStr);

		var elem = document.getElementById("splatMenu");
		if (displayMolecules)
			elem.style.display = "block";
		else
			elem.style.display = "none";
	}

	//VIEW
	var viewMenu = mainUL.append("li")
		.attr("class", "dropdown");

	viewMenu.append("a")
		.attr("class", "dropdown-toggle")
		.attr("data-toggle", "dropdown")
		.style('cursor', 'pointer')
		.html('View <span class="caret"></span>');

	var viewDropdown = viewMenu.append("ul")
		.attr("class", "dropdown-menu");

	if (has_webgl) {
		if (va_count == 1 || composite_view) {
			var htmlStr = '<i class="material-icons">3d_rotation</i> 3D surface';
			viewDropdown.append("li")
				.append("a")
				.style('cursor', 'pointer')
				.on("click", function () {
					init_surface();

				})
				.html(htmlStr);
		}
	}
	else {
		viewDropdown.append("li")
			.append("a")
			.attr("disabled", "disabled")
			.style("font-style", "italic")
			.style('cursor', 'not-allowed')
			.html('<span class="fas fa-eye-slash"></span> WebGL not enabled, disabling 3D surface');
	}

	if (va_count > 1 && va_count <= 3) {
		htmlStr = composite_view ? '<span class="fas fa-check-square"></span> RGB composite mode' : '<span class="far fa-square"></span> RGB composite mode';
		viewDropdown.append("li")
			.append("a")
			.attr("id", "displayComposite")
			.style('cursor', 'pointer')
			.on("click", function () {
				composite_view = !composite_view;
				var htmlStr = composite_view ? '<span class="fas fa-check-square"></span> RGB composite mode' : '<span class="far fa-square"></span> RGB composite mode';
				d3.select(this).html(htmlStr);

				var loc = window.location.href.replace("&view=composite", "");

				if (composite_view)
					window.location.replace(loc + "&view=composite");
				else
					window.location.replace(loc);

				/*var new_loc = window.location.href.replace("&view=", "&dummy=");

				if (composite_view && optical_view)
					new_loc += "&view=composite,optical";
				else {
					if (composite_view)
						new_loc += "&view=composite";

					if (optical_view)
						new_loc += "&view=optical";
				}

				window.location.replace(new_loc);*/
			})
			.html(htmlStr);
	}

	if (va_count == 1 || composite_view) {
		htmlStr = displayContours ? '<span class="fas fa-check-square"></span> contour lines' : '<span class="far fa-square"></span> contour lines';
		viewDropdown.append("li")
			.append("a")
			.attr("id", "displayContours")
			.style('cursor', 'pointer')
			.on("click", function () {
				displayContours = !displayContours;
				var htmlStr = displayContours ? '<span class="fas fa-check-square"></span> contour lines' : '<span class="far fa-square"></span> contour lines';
				d3.select(this).html(htmlStr);
				//var elem = d3.selectAll("#contourPlot");

				if (displayContours) {
					d3.select('#contour_control_li').style("display", "block");
				}
				else {
					d3.select('#contour_control_li').style("display", "none");
				}

				if (displayContours) {
					document.getElementById("ContourSVG").style.display = "block";
					//elem.attr("opacity",1);

					//if(document.getElementById('contourPlot') == null)
					if (!has_contours)
						update_contours();
				}
				else {
					document.getElementById("ContourSVG").style.display = "none";
					//elem.attr("opacity",0);
				}
			})
			.html(htmlStr);
	}

	if (va_count == 1 || composite_view) {
		htmlStr = displayGridlines ? '<span class="fas fa-check-square"></span> lon/lat grid lines' : '<span class="far fa-square"></span> lon/lat grid lines';
		viewDropdown.append("li")
			.append("a")
			.attr("id", "displayGridlines")
			.style('cursor', 'pointer')
			.on("click", function () {
				displayGridlines = !displayGridlines;
				localStorage_write_boolean("displayGridlines", displayGridlines);
				var htmlStr = displayGridlines ? '<span class="fas fa-check-square"></span> lon/lat grid lines' : '<span class="far fa-square"></span> lon/lat grid lines';
				d3.select(this).html(htmlStr);
				var elem = d3.select("#gridlines");
				if (displayGridlines)
					elem.attr("opacity", 1);
				else
					elem.attr("opacity", 0);
			})
			.html(htmlStr);

		htmlStr = displayLegend ? '<span class="fas fa-check-square"></span> image legend' : '<span class="far fa-square"></span> image legend';
		viewDropdown.append("li")
			.append("a")
			.style('cursor', 'pointer')
			.on("click", function () {
				displayLegend = !displayLegend;
				localStorage_write_boolean("displayLegend", displayLegend);
				var htmlStr = displayLegend ? '<span class="fas fa-check-square"></span> image legend' : '<span class="far fa-square"></span> image legend';
				d3.select(this).html(htmlStr);

				if (va_count == 1) {
					var elem = d3.select("#legend");

					if (displayLegend)
						elem.attr("opacity", 1);
					else
						elem.attr("opacity", 0);
				}
				else {
					for (let index = 1; index <= va_count; index++) {
						var elem = d3.select("#legend" + index);

						if (displayLegend)
							elem.attr("opacity", 1);
						else
							elem.attr("opacity", 0);
					}
				}
			})
			.html(htmlStr);
	}

	if (!optical_view) {
		htmlStr = displayMolecules ? '<span class="fas fa-check-square"></span> spectral lines' : '<span class="far fa-square"></span> spectral lines';
		viewDropdown.append("li")
			.append("a")
			.style('cursor', 'pointer')
			.on("click", function () {
				displayMolecules = !displayMolecules;
				localStorage_write_boolean("displayMolecules", displayMolecules);
				var htmlStr = displayMolecules ? '<span class="fas fa-check-square"></span> spectral lines' : '<span class="far fa-square"></span> spectral lines';
				d3.select(this).html(htmlStr);
				var elem = d3.select("#molecules");
				if (displayMolecules)
					elem.attr("opacity", 1);
				else
					elem.attr("opacity", 0);

				var elem = document.getElementById("splatMenu");
				if (displayMolecules)
					elem.style.display = "block";
				else
					elem.style.display = "none";
			})
			.html(htmlStr);

		htmlStr = displaySpectrum ? '<span class="fas fa-check-square"></span> spectrum' : '<span class="far fa-square"></span> spectrum';
		viewDropdown.append("li")
			.append("a")
			.style('cursor', 'pointer')
			.on("click", function () {
				displaySpectrum = !displaySpectrum;
				localStorage_write_boolean("displaySpectrum", displaySpectrum);
				var htmlStr = displaySpectrum ? '<span class="fas fa-check-square"></span> spectrum' : '<span class="far fa-square"></span> spectrum';
				d3.select(this).html(htmlStr);
				var elem = document.getElementById("SpectrumCanvas");
				if (displaySpectrum) {
					elem.style.display = "block";
					d3.select("#yaxis").attr("opacity", 1);
					d3.select("#ylabel").attr("opacity", 1);
				}
				else {
					elem.style.display = "none";
					d3.select("#yaxis").attr("opacity", 0);
					d3.select("#ylabel").attr("opacity", 0);
				}
			})
			.html(htmlStr);

		if (va_count == 1 || composite_view) {
			htmlStr = displayBeam ? '<span class="fas fa-check-square"></span> telescope beam' : '<span class="far fa-square"></span> telescope beam';
			viewDropdown.append("li")
				.append("a")
				.attr("id", "displayBeam")
				.style('cursor', 'pointer')
				.on("click", function () {
					displayBeam = !displayBeam;
					var htmlStr = displayBeam ? '<span class="fas fa-check-square"></span> telescope beam' : '<span class="far fa-square"></span> telescope beam';
					d3.select(this).html(htmlStr);

					if (displayBeam) {
						d3.select("#beam").attr("opacity", 1);
						d3.select("#zoomBeam").attr("opacity", 1);
					}
					else {
						d3.select("#beam").attr("opacity", 0);
						d3.select("#zoomBeam").attr("opacity", 0);
					}
				})
				.html(htmlStr);
		}
	}

	//HELP
	var rightUL = main.append("ul")
		.attr("class", "nav navbar-nav navbar-right");

	var helpMenu = rightUL.append("li")
		.attr("class", "dropdown");

	helpMenu.append("a")
		.attr("class", "dropdown-toggle")
		.attr("data-toggle", "dropdown")
		.style('cursor', 'pointer')
		.html('<span class="fas fa-question-circle"></span> Help <span class="caret"></span>');

	var helpDropdown = helpMenu.append("ul")
		.attr("class", "dropdown-menu");

	helpDropdown.append("li")
		.append("a")
		.style('cursor', 'pointer')
		.on("click", show_help)
		.html('user guide <span class="fas fa-wrench"></span>');

	helpDropdown.append("li")
		.append("a")
		.attr("href", "mailto:help_desk@jvo.nao.ac.jp?subject=" + votable.getAttribute('data-server-string') + " feedback [" + votable.getAttribute('data-server-version') + "/" + get_js_version() + "]")
		.html('send feedback');

	helpDropdown.append("li")
		.append("a")
		.style("color", "#336699")
		.html("[" + votable.getAttribute('data-server-version') + "/" + get_js_version() + "]");
}

function show_help() {
	$("#help").modal("show");

	var modal = document.getElementById('help');
	var span = document.getElementById('helpclose');

	// When the user clicks on <span> (x), close the modal
	span.onclick = function () {
		$("#help").modal("hide");
	}
	// When the user clicks anywhere outside of the modal, close it
	window.onclick = function (event) {
		if (event.target == modal) {
			$("#help").modal("hide");
		}
	}
}

function donotshow() {
	var checkbox = document.getElementById('donotshowcheckbox');

	localStorage_write_boolean("welcome_v4", !checkbox.checked);
};

function show_timeout() {
	try {
		$('#welcomeScreen').modal('hide');
	}
	catch (e) { };

	var div = d3.select("body")
		.append("div")
		.attr("class", "container timeout");

	var title = div.append("h1")
		.style("margin-top", "20%")
		.attr("align", "center")
		.text("60 min. inactivity time-out");

	div.append("h2")
		.attr("align", "center")
		.text("PLEASE RELOAD THE PAGE");
}

function show_critical_error() {
	try {
		$('#welcomeScreen').modal('hide');
	}
	catch (e) { };

	var div = d3.select("body")
		.append("div")
		.attr("class", "container timeout");

	var title = div.append("h1")
		.style("margin-top", "25%")
		.style("color", "red")
		.attr("align", "center")
		.text("CRITICAL ERROR");

	div.append("h2")
		.attr("align", "center")
		//.style("color", "red")
		.append("a")
		.attr("class", "links")
		.attr("href", "mailto:help_desk@jvo.nao.ac.jp?subject=" + votable.getAttribute('data-server-string') + " error [" + votable.getAttribute('data-server-version') + "/" + get_js_version() + "]&body=Error accessing " + datasetId)
		.html('PLEASE INFORM AN ADMINISTRATOR');
}

function show_unsupported_media_type() {
	try {
		$('#welcomeScreen').modal('hide');
	}
	catch (e) { };

	var div = d3.select("body")
		.append("div")
		.attr("class", "container timeout");

	var title = div.append("h1")
		.style("margin-top", "25%")
		.style("color", "red")
		.attr("align", "center")
		.text("UNSUPPORTED MEDIA TYPE");

	div.append("h2")
		.attr("align", "center")
		//.style("color", "red")
		.text("FITSWEBQL SUPPORTS ONLY FITS DATA");
}

function show_not_found() {
	try {
		$('#welcomeScreen').modal('hide');
	}
	catch (e) { };

	var div = d3.select("body")
		.append("div")
		.attr("class", "container timeout");

	var title = div.append("h1")
		.style("margin-top", "20%")
		.style("color", "red")
		.attr("align", "center")
		.text("DATA NOT FOUND ON THE REMOTE SITE");

	div.append("h2")
		.attr("align", "center")
		//.style("color", "red")
		.text("THE FITS FILE CANNOT BE FOUND");

	div.append("h2")
		.attr("align", "center")
		//.style("color", "red")
		.text("AND/OR");

	div.append("h2")
		.attr("align", "center")
		//.style("color", "red")
		.text("THE REMOTE URL MAY BE INCORRECT/OUT-OF-DATE");

}

function show_welcome() {
	var div = d3.select("body")
		.append("div")
		.attr("class", "container")
		.append("div")
		.attr("id", "welcomeScreen")
		.attr("class", "modal modal-center")
		.attr("role", "dialog")
		.append("div")
		.attr("class", "modal-dialog");

	var contentDiv = div.append("div")
		.attr("class", "modal-content");

	var headerDiv = contentDiv.append("div")
		.attr("class", "modal-header");

	headerDiv.append("button")
		.attr("type", "button")
		.attr("data-dismiss", "modal")
		.attr("id", "welcomeclose")
		.attr("class", "close")
		.style("color", "red")
		.text("×");

	headerDiv.append("h2")
		.attr("align", "center")
		.html('WELCOME TO FITSWEBQL <SUB><SMALL>26</SMALL></SUB>Fe');

	var bodyDiv = contentDiv.append("div")
		.attr("id", "modal-body")
		.attr("class", "modal-body");

	bodyDiv.append("h3")
		.text("What's New");

	var ul = bodyDiv.append("ul")
		.attr("class", "list-group");

	ul.append("li")
		.attr("class", "list-group-item list-group-item-success")
		.html('<h4>CSV spectrum export back-ported from <a href="https://github.com/jvo203/FITSWEBQLSE"><em>FITSWEBQLSE</em></a></h4>');


	ul.append("li")
		.attr("class", "list-group-item list-group-item-success")
		.html('<h4>Server-side code changed from C/C++ to <a href="https://www.rust-lang.org"><em>Rust</em></a></h4>');

	ul.append("li")
		.attr("class", "list-group-item list-group-item-success")
		.html('<h4>Images encoded as <a  href="https://en.wikipedia.org/wiki/VP9"><em>Google VP9</em></a> keyframes</h4>');

	ul.append("li")
		.attr("class", "list-group-item list-group-item-success")
		.html('<h4><a  href="https://en.wikipedia.org/wiki/High_Efficiency_Video_Coding"><em>HEVC</em></a> streaming video for FITS data cubes</h4>');

	ul.append("li")
		.attr("class", "list-group-item list-group-item-success")
		.html('<h4><h4><a href="https://en.wikipedia.org/wiki/WebAssembly"><em>WebAssembly</em></a>-accelerated HTML Video Canvas</h4>');

	let textColour = 'yellow';

	if (theme == 'bright')
		textColour = 'red';

	if (!isLocal) {
		ul.append("li")
			.attr("class", "list-group-item list-group-item-success")
			.html('<h4>FITSWebQL Personal Edition (local desktop) on GitHub: <a href="https://github.com/jvo203/fits_web_ql"><em>fits_web_ql installation instructions</em></a></h4>');
	}

	bodyDiv.append("h3")
		.text("Browser recommendation");

	if (!wasm_supported) {
		bodyDiv.append("p")
			.html('A modern browser with <a href="https://en.wikipedia.org/wiki/WebAssembly" style="color:' + textColour + '"><b>WebAssembly (Wasm)</b></a> support is required.');
	}

	bodyDiv.append("p")
		.html('For optimum performance we recommend <a href="https://www.google.com/chrome/index.html" style="color:' + textColour + '"><b>Google Chrome</b></a>. Firefox Quantum is pretty much OK. Safari on MacOS works. We do NOT recommend IE.');

	//bodyDiv.append("hr");    

	var footer = contentDiv.append("div")
		.attr("class", "modal-footer");

	/*footer.append("button")	
	.attr("type", "button")
	.attr("data-dismiss", "modal")
		.attr("class", "button btn-lg pull-right")
	.attr("align","center")
	.html('<span class="fas fa-times"></span> Close') ;*/

	var href = "mailto:help_desk@jvo.nao.ac.jp?subject=" + votable.getAttribute('data-server-string') + " bug report [" + votable.getAttribute('data-server-version') + "/" + get_js_version() + "]";

	footer.append("p")
		//.style("color", "#a94442")
		.attr("align", "left")
		.html('<label style="cursor: pointer"><input type="checkbox" value="" class="control-label" style="cursor: pointer" id="donotshowcheckbox" onchange="javascript:donotshow();">&nbsp;don\'t show this dialogue again</label>' + '&nbsp;&nbsp;&nbsp;<a style="color:red" href="' + href + '">page loading problems?</a>' + '<button type="submit" class="btn btn-danger btn-default pull-right" data-dismiss="modal"><span class="fas fa-times"></span> Close</button>');

	$('#welcomeScreen').modal('show');
}

function setup_help() {
	var div = d3.select("body")
		.append("div")
		.attr("class", "container")
		.append("div")
		.attr("id", "help")
		.attr("class", "modal fade")
		.attr("role", "dialog")
		.append("div")
		.attr("class", "modal-dialog");

	var contentDiv = div.append("div")
		.attr("class", "modal-content");

	var headerDiv = contentDiv.append("div")
		.attr("class", "modal-header");

	headerDiv.append("span")
		.attr("id", "helpclose")
		.attr("class", "close")
		.style("color", "red")
		.text("×");

	var title = headerDiv.append("h2")
		.text("FITSWebQL HOW-TO");

	var bodyDiv = contentDiv.append("div")
		.attr("id", "modal-body")
		.attr("class", "modal-body");

	bodyDiv.append("h3")
		.attr("id", "h3")
		.text("Spectrum Export (back-ported from v5)");

	bodyDiv.append("p")
		.html("The current image/viewport spectrum can be exported to a <b>CSV</b> file");

	bodyDiv.append("p")
		.html("Other formats, e.g., <em>JSON</em>, <em>PLAIN TEXT</em> or <em>FITS</em> are under consideration");

	var csv = bodyDiv.append("video")
		.attr("width", "100%")
		.attr("controls", "")
		.attr("preload", "metadata");

	csv.append("source")
		.attr("src", "https://cdn.jsdelivr.net/gh/jvo203/FITSWEBQLSE/htdocs/fitswebql/spectrum_export.mp4");

	csv.append("p")
		.html("Your browser does not support the video tag.");

	bodyDiv.append("hr");

	bodyDiv.append("h3")
		.text("3D View");

	bodyDiv.append("p")
		.html("An <span style=\"color:#a94442\">experimental</span> WebGL feature resulting in high memory consumption. After using it a few times a browser may run out of memory.");

	bodyDiv.append("p")
		.html("Reloading a page should fix the problem");

	/*bodyDiv.append("p")
	.html("To enable it check <i>Preferences</i>/<i>3D View (experimental)</i> and a \"3D View\" button should appear towards the bottom of the page") ;*/

	bodyDiv.append("p")
		.html("To view a 3D surface of the FITS cube image, click <i>3D surface</i> in the <i>View</i> menu");

	bodyDiv.append("hr");

	bodyDiv.append("h3")
		.attr("id", "h3")
		.text("Realtime Spectrum Updates");

	bodyDiv.append("p")
		.html("<i>Preferences/realtime spectrum updates</i> works best over <i>low-latency</i> network connections");

	bodyDiv.append("p")
		.html("<i>Kalman Filter</i> is used to predict the mouse movement after taking into account a latency of a network connection to Japan");

	bodyDiv.append("p")
		.html("when disabled the spectrum refresh will be requested after a 250ms delay since the last movement of the mouse");

	bodyDiv.append("hr");


	bodyDiv.append("h3")
		.attr("id", "h3")
		.text("Realtime FITS Cube Video Updates");

	bodyDiv.append("p")
		.html("<i>Preferences/realtime video updates</i> works best over <i>low-latency</i> network connections with available bandwidth <i>over 1 mbps</i>");

	bodyDiv.append("p")
		.html("when disabled the FITS cube video frame will be requested after a 250ms delay since the last movement of the mouse");

	bodyDiv.append("p")
		.html('<span class="fas fa-play"></span>&nbsp; replay period 10s');

	bodyDiv.append("p")
		.html('<span class="fas fa-forward"></span>&nbsp; replay period 5s');

	bodyDiv.append("p")
		.html('<span class="fas fa-fast-forward"></span>&nbsp; replay period 2.5s');

	bodyDiv.append("hr");

	bodyDiv.append("h3")
		.text("Zoom In/Out of region");

	bodyDiv.append("p")
		.html("scroll mouse wheel up/down (<i>mouse</i>)");

	bodyDiv.append("p")
		.html("move two fingers up/down (<i>touchpad</i>)");

	bodyDiv.append("hr");

	bodyDiv.append("h3")
		.text("Copy RA/DEC");

	bodyDiv.append("p")
		.html("<b>Ctrl + C</b>");

	bodyDiv.append("hr");

	bodyDiv.append("h3")
		.text("Save region as FITS");

	bodyDiv.append("p")
		.html("<b>Ctrl + S</b> (<i>keyboard</i>)");

	bodyDiv.append("p")
		.html("drag over main image (<i>mouse</i>)");

	bodyDiv.append("hr");

	bodyDiv.append("h3")
		.text("Show Frequency/Velocity/Molecular Information");

	bodyDiv.append("p")
		.html("<b>hover</b> a mouse over X-axis");

	bodyDiv.append("hr");

	bodyDiv.append("h3")
		.text("Skip to the Next Molecular Line");

	bodyDiv.append("p")
		.html("press <b>&larr;</b> or <b>&rarr;</b> whilst <b>hovering</b> over X-axis");

	bodyDiv.append("hr");

	bodyDiv.append("h3")
		.text("Jump to Splatalogue");

	bodyDiv.append("p")
		.html("press <b>Enter</b> whilst <b>hovering</b> over X-axis");

	bodyDiv.append("hr");

	bodyDiv.append("h3")
		.text("Select Frequency Range");

	bodyDiv.append("p")
		.html("<b>drag</b> over X-axis");

	bodyDiv.append("hr");

	bodyDiv.append("h3")
		.text("Set REST Frequency");

	bodyDiv.append("p")
		.html("press <b>f</b> over X-axis; for detailed information see the&nbsp;")
		.append("a")
		.attr("class", "links")
		.attr("href", "relative velocity.pdf")
		.attr("target", "_blank")
		.style("target-new", "tab")
		.html("<u>relative velocity guide</u>");

	bodyDiv.append("hr");

	bodyDiv.append("h3")
		.text("Temporarily Fix Y-Axis Range");

	bodyDiv.append("p")
		.html("press <b>s</b> over main image");

	bodyDiv.append("h4")
		.text("adjust the fixed Y-Axis range");

	bodyDiv.append("p")
		.html("move mouse cursor over to the Y-Axis whilst holding the 「Shift」 key");

	bodyDiv.append("p")
		.html("drag the mouse over the Y-Axis to <i>shift</i> it <em>UP</em> and <em>DOWN</em>");

	bodyDiv.append("p")
		.html("use the mouse <i>scroll wheel</i> or a two-finger <i>touch gesture</i> to <i>re-scale</i> the Y-Axis range");

	var vid = bodyDiv.append("video")
		.attr("width", "100%")
		.attr("controls", "")
		.attr("preload", "metadata");

	vid.append("source")
		.attr("src", "https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/fixed_scale_y_axis.mp4");

	vid.append("p")
		.html("Your browser does not support the video tag.");

	bodyDiv.append("hr");

	bodyDiv.append("h3")
		.text("Hold current view region");

	bodyDiv.append("p")
		.html("keep pressing <b>↑Shift</b>");

	bodyDiv.append("hr");

	bodyDiv.append("h3")
		.text("Print");

	bodyDiv.append("p")
		.html("in a browser <i>File/Print Preview</i>, adjust scale as needed (i.e. 25% or 50%)");

	bodyDiv.append("hr");

	bodyDiv.append("h3")
		.text("browser support:");

	bodyDiv.append("p")
		.text("Chrome ◯, Firefox △, Safari ◯, MS Edge △, IE11 ×");

	var footer = contentDiv.append("div")
		.attr("class", "modal-footer");

	if (!isLocal) {
		footer.append("h3")
			.text("FITSWebQL Personal Edition:");

		let textColour = 'yellow';

		if (theme == 'bright')
			textColour = 'red';

		footer.append("p")
			.html("A local version is available on GitHub: ")
			.append("a")
			.style("color", textColour)
			.attr("href", "https://github.com/jvo203/fits_web_ql")
			.attr("target", "_blank")
			.style("target-new", "tab")
			.html("<b>fits_web_ql installation instructions</b>");
	}

	footer.append("h3")
		.text("CREDITS:");

	footer.append("p")
		.text("Site design Ⓒ Christopher A. Zapart @ NAOJ, 2015 - 2018. JavaScript RA/DEC conversion Ⓒ Robert Martin Ayers, 2009, 2011, 2014.");

	footer.append("h3")
		.text("VERSION:");

	footer.append("p")
		.text(votable.getAttribute('data-server-version') + "/" + get_js_version());
}

function setup_FITS_header_page() {
	var div = d3.select("body")
		.append("div")
		.attr("class", "container")
		.append("div")
		.attr("id", "fitsHeader")
		.attr("class", "modal fade")
		.attr("role", "dialog")
		.append("div")
		.attr("class", "modal-dialog");

	var contentDiv = div.append("div")
		.attr("class", "modal-content");

	var headerDiv = contentDiv.append("div")
		.attr("class", "modal-header");

	headerDiv.append("span")
		.attr("id", "fitsHeaderClose")
		.attr("class", "close")
		.style("color", "red")
		.text("×");

	var title = headerDiv.append("h3")
		.text("FITS HEADER");

	var bodyDiv = contentDiv.append("div")
		.attr("id", "modal-body")
		.attr("class", "modal-body");

	if (va_count > 1) {
		var ul = bodyDiv.append("ul")
			.attr("class", "nav nav-tabs");

		for (let index = 1; index <= va_count; index++) {
			let classStr = '';

			if (index == 1)
				classStr = 'active';

			var li = ul.append("li")
				.attr("class", classStr);

			var a = li.append("a")
				.attr("id", "headerTag#" + index)
				.attr("data-toggle", "tab")
				.attr("href", "#header" + index)
				.style("font-size", "125%")
				.style("font-weight", "bold")
				.html(datasetId[index - 1]);
		}

		var div = bodyDiv.append("div")
			.attr("class", "tab-content");

		for (let index = 1; index <= va_count; index++) {
			let classStr = 'tab-pane fade';

			if (index == 1)
				classStr += ' in active';

			var tab = div.append("div")
				.attr("id", "header" + index)
				.attr("class", classStr);

			var p = tab.append("p")
				.attr("id", "headerText#" + index);

			var it = p.append("I")
				.text("FITS HEADER data not transmitted yet. Please try later.");
		}
	}
	else {
		var p = bodyDiv.append("p")
			.attr("id", "headerText#" + va_count);

		var it = p.append("I")
			.text("FITS HEADER data not transmitted yet. Please try later.");
	}
}

function display_FITS_header(index) {
	let fitsData = fitsContainer[index - 1];
	let fitsHeader = fitsData.HEADER;

	//probably there is no need for 'try' as the LZ4 decompressor has been removed
	//decompression is now handled by the browser behind the scenes
	try {
		var headerText = document.getElementById('headerText#' + index);
		headerText.innerHTML = fitsHeader.trim().replace(/(.{80})/g, "$1<br>");

		var headerTag = document.getElementById('headerTag#' + index);

		let line = fitsData.LINE.trim();

		if (line != "")
			headerTag.innerHTML = plain2chem(line, true);
	}
	catch (e) {
		console.log(e);
	};
}

function display_range_validation() {
	var div = d3.select("body")
		.append("div")
		.attr("class", "container")
		.append("div")
		.attr("id", "rangevalidation")
		.attr("class", "modal fade")
		.attr("role", "dialog")
		.append("div")
		.attr("class", "modal-dialog");

	var contentDiv = div.append("div")
		.attr("class", "modal-content")
		.style("margin", "25% auto");

	var headerDiv = contentDiv.append("div")
		.attr("class", "modal-header");

	headerDiv.append("span")
		.attr("id", "rangevalidationclose")
		.attr("class", "close")
		.style("color", "red")
		.text("×");

	headerDiv.append("h3")
		.style("color", "#a94442")
		.attr("align", "center")
		.text("INPUT ERROR");

	var bodyDiv = contentDiv.append("div")
		.attr("id", "modal-body")
		.attr("class", "modal-body");

	bodyDiv.append("p")
		.html("Incorrect velocity/redshift input. Valid values are |V| < c and z > -1.");
}

function download_confirmation() {
	var div = d3.select("body")
		.append("div")
		.attr("class", "container")
		.append("div")
		.attr("id", "downloadConfirmation")
		.attr("class", "modal fade")
		.attr("role", "dialog")
		.append("div")
		.attr("class", "modal-dialog");

	var contentDiv = div.append("div")
		.attr("class", "modal-content")
		.style("margin", "25% auto");

	var headerDiv = contentDiv.append("div")
		.attr("class", "modal-header");

	headerDiv.append("span")
		.attr("id", "downloadconfirmationclose")
		.attr("class", "close")
		.style("color", "red")
		.text("×");

	headerDiv.append("h3")
		.style("color", "#a94442")
		.attr("align", "center")
		.text("CONFIRMATION");

	var bodyDiv = contentDiv.append("div")
		.attr("id", "modal-body")
		.attr("class", "modal-body");

	bodyDiv.append("p")
		.html("Selecting a sub-region triggers a corresponding partial FITS file download.");

	var p = bodyDiv.append("p")
		.html("Proceed with the download?");

	p.append("span")
		.html("&nbsp;&nbsp;&nbsp;");

	p.append("button")
		.attr("id", "okButton")
		.attr("type", "button")
		.attr("class", "btn btn-primary")
		.attr("onclick", "ok_download()")
		.html("&nbsp; Yes &nbsp;");

	p.append("span")
		.html("&nbsp;&nbsp;&nbsp;");

	p.append("button")
		.attr("id", "cancelButton")
		.attr("type", "button")
		.attr("class", "btn btn-default")
		.attr("onclick", "cancel_download()")
		//.attr("autofocus","")
		.html("&nbsp; Cancel &nbsp;");

	var footer = contentDiv.append("div")
		.attr("class", "modal-footer");

	footer.append("p")
		.style("color", "#a94442")
		.html("you can disable showing this dialog via the <i>Preferences</i> menu, <i>download confirmation</i> checkbox");

	show_download_confirmation();
	$('#downloadConfirmation').modal('show');
}

function display_rgb_legend() {
	console.log("display_rgb_legend()");

	if (va_count > 3)
		return;

	for (let index = 1; index <= va_count; index++) {
		//do we have all the inputs?
		var black, white, median, multiplier, flux;

		var flux_elem = d3.select("#flux_path" + index);

		try {
			flux = document.getElementById('flux' + index).value
		}
		catch (e) {
			console.log('flux not available yet');
			return;
		};

		try {
			black = parseFloat(flux_elem.attr("black"));
		}
		catch (e) {
			console.log('black not available yet');
			return;
		};

		try {
			white = parseFloat(flux_elem.attr("white"));
		}
		catch (e) {
			console.log('white not available yet');
			return;
		};

		try {
			median = parseFloat(flux_elem.attr("median"));
		}
		catch (e) {
			console.log('median not available yet');
			return;
		};

		multiplier = get_noise_sensitivity(noise_sensitivity);

		try {
			d3.select("#legend" + index).remove();
		}
		catch (e) {
		}

		var svg = d3.select("#BackgroundSVG");
		var width = parseFloat(svg.attr("width"));
		var height = parseFloat(svg.attr("height"));

		var divisions = 64;//100
		var legendHeight = 0.8 * height;
		var rectHeight = legendHeight / divisions;
		var rectWidth = 5 * rectHeight;//0.05*width;
		var newData = [];

		if (imageContainer[index - 1] == null) {
			console.log("no imageContainer element @", index - 1);
			return;
		}

		let image_bounding_dims = imageContainer[index - 1].image_bounding_dims;
		let pixel_range = imageContainer[index - 1].pixel_range;
		let min_pixel = pixel_range.min_pixel;
		let max_pixel = pixel_range.max_pixel;

		let scale = get_image_scale(width, height, image_bounding_dims.width, image_bounding_dims.height);
		scale = 2.0 * scale / va_count;

		let img_width = scale * image_bounding_dims.width;
		let img_height = scale * image_bounding_dims.height;

		for (var i = 0; i < divisions; i++)
			newData.push(min_pixel + (max_pixel - min_pixel) * i / (divisions - 1));

		//var x = Math.max(0.05*width, (width+img_width)/2 + 0.5*rectWidth);
		//var x = Math.max(0.05*width + (index-1)*1.5*rectWidth, (width-img_width)/2 - va_count*2.4*rectWidth + (index-1)*1.5*rectWidth);
		var x = (width - img_width) / 2 - 0.05 * width - (va_count + 1.5 - index) * 1.5 * rectWidth;

		var group = svg.append("g")
			.attr("id", "legend" + index)
			.attr("opacity", 1.0);

		let strokeColour = 'white';

		if (theme == 'bright')
			strokeColour = 'black';

		let rgb = ['red', 'green', 'blue'];

		group.selectAll('rect')
			.data(newData)
			.enter()
			.append('rect')
			.attr("x", x)
			.attr("y", function (d, i) { return (0.9 * height - (i + 1) * rectHeight); })
			.attr("height", (rectHeight + 1))
			.attr("width", rectWidth)
			//.attr("stroke", strokeColour)
			//.attr("stroke-width", 0.1)
			.attr("stroke", "none")
			//.attr('fill', function(d, i) { return pixel2rgba(1.0*d, index-1, 0.8);});
			.attr('fill', function (d, i) { return interpolate_colourmap(d, rgb[index - 1], 0.8); });

		var colourScale = d3.scaleLinear()
			.range([0.8 * height, 0])
			.domain([0, 1]);

		var colourAxis = d3.axisRight(colourScale)
			.tickSizeOuter([0])
			.tickSizeInner([0])
			.tickFormat(function (d) {
				var prefix = "";

				if (d == 0)
					prefix = "≤";

				if (d == 1)
					prefix = "≥";

				var pixel = min_pixel + d * (max_pixel - min_pixel);
				var pixelVal = get_pixel_flux(pixel, index);

				var number;

				if (Math.abs(pixelVal) <= 0.001 || Math.abs(pixelVal) >= 1000)
					number = pixelVal.toExponential(3);
				else
					number = pixelVal.toPrecision(3);

				return prefix + number;
			});

		group.append("g")
			.attr("class", "colouraxis")
			.attr("id", "legendaxis")
			.style("stroke-width", emStrokeWidth)
			.attr("transform", "translate(" + x + "," + 0.1 * height + ")")
			.call(colourAxis);

		let fitsData = fitsContainer[index - 1];

		var bunit = '';
		if (fitsData.BUNIT != '') {
			bunit = fitsData.BUNIT.trim();

			if (fitsData.depth > 1 && has_velocity_info)
				bunit += '•km/s';

			bunit = "[" + bunit + "]";
		}

		let line = fitsData.LINE.trim();
		let filter = fitsData.FILTER.trim();

		if (filter != "")
			line = filter;
		else {
			if (line == "")
				line = "line-" + index;
		}

		group.append("foreignObject")
			.attr("x", (x + 0.0 * rectWidth))
			.attr("y", 0.9 * height + 0.75 * emFontSize)
			.attr("width", 5 * emFontSize)
			.attr("height", 3 * emFontSize)
			.append("xhtml:div")
			.html('<p style="text-align: left">' + plain2chem(line, false) + '&nbsp;' + bunit + '</p>');
	}

	if (va_count == 1) {
		var elem = d3.select("#legend");

		if (displayLegend)
			elem.attr("opacity", 1);
		else
			elem.attr("opacity", 0);
	}
	else {
		for (let index = 1; index <= va_count; index++) {
			var elem = d3.select("#legend" + index);

			if (displayLegend)
				elem.attr("opacity", 1);
			else
				elem.attr("opacity", 0);
		}
	}
}

function display_legend() {
	console.log("display_legend()");

	if (va_count > 1)
		return;

	//do we have all the inputs?
	var black, white, median, multiplier, flux;

	var flux_elem = d3.select("#flux_path" + va_count);

	try {
		flux = document.getElementById('flux' + va_count).value
	}
	catch (e) {
		console.log('flux not available yet');
		return;
	};

	try {
		black = parseFloat(flux_elem.attr("black"));
	}
	catch (e) {
		console.log('black not available yet');
		return;
	};

	try {
		white = parseFloat(flux_elem.attr("white"));
	}
	catch (e) {
		console.log('white not available yet');
		return;
	};

	try {
		median = parseFloat(flux_elem.attr("median"));
	}
	catch (e) {
		console.log('median not available yet');
		return;
	};

	multiplier = get_noise_sensitivity(noise_sensitivity);

	var rect = d3.select("#image_rectangle");

	var img_width, img_height;

	try {
		img_width = parseFloat(rect.attr("width"));
		img_height = parseFloat(rect.attr("height"));
	}
	catch (e) {
		console.log('image_rectangle not available yet');
		return;
	}

	try {
		d3.select("#legend").remove();
	}
	catch (e) {
	}

	var svg = d3.select("#BackgroundSVG");
	var width = parseFloat(svg.attr("width"));
	var height = parseFloat(svg.attr("height"));

	var divisions = 64;//100
	var legendHeight = 0.8 * height;
	var rectHeight = legendHeight / divisions;
	var rectWidth = 5 * rectHeight;//0.05*width;
	var newData = [];

	var pixel_range = imageContainer[va_count - 1].pixel_range;
	var min_pixel = pixel_range.min_pixel;
	var max_pixel = pixel_range.max_pixel;

	for (var i = 0; i < divisions; i++)
		newData.push(min_pixel + (max_pixel - min_pixel) * i / (divisions - 1));

	var x = Math.max(0.05 * width, (width - img_width) / 2 - 1.5 * rectWidth);

	var group = svg.append("g")
		.attr("id", "legend")
		.attr("opacity", 1.0);

	let strokeColour = 'white';

	if (theme == 'bright')
		strokeColour = 'black';

	group.selectAll('rect')
		.data(newData)
		.enter()
		.append('rect')
		.attr("x", x)
		.attr("y", function (d, i) { return (0.9 * height - (i + 1) * rectHeight); })
		.attr("height", (rectHeight + 1))
		.attr("width", rectWidth)
		//.attr("stroke", strokeColour)
		//.attr("stroke-width", 0.1)
		.attr("stroke", "none")
		.attr('fill', function (d, i) { return interpolate_colourmap(d, colourmap, 1.0); });

	var colourScale = d3.scaleLinear()
		.range([0.8 * height, 0])
		.domain([0, 1]);

	var colourAxis = d3.axisRight(colourScale)
		.tickSizeOuter([0])
		.tickSizeInner([0])
		.tickFormat(function (d) {
			var prefix = "";

			if (d == 0)
				prefix = "≤";

			if (d == 1)
				prefix = "≥";

			var pixel = min_pixel + d * (max_pixel - min_pixel);
			var pixelVal = get_pixel_flux(pixel, va_count);

			var number;

			if (Math.abs(pixelVal) <= 0.001 || Math.abs(pixelVal) >= 1000)
				number = pixelVal.toExponential(3);
			else
				number = pixelVal.toPrecision(3);

			return prefix + number;
		});

	group.append("g")
		.attr("class", "colouraxis")
		.attr("id", "legendaxis")
		.style("stroke-width", emStrokeWidth)
		.attr("transform", "translate(" + ((width - img_width) / 2 - 1.5 * rectWidth) + "," + 0.1 * height + ")")
		.call(colourAxis);

	let fitsData = fitsContainer[va_count - 1];

	var bunit = '';
	if (fitsData.BUNIT != '') {
		bunit = fitsData.BUNIT.trim();

		if (fitsData.depth > 1 && has_velocity_info)
			bunit += '•km/s';

		bunit = "[" + bunit + "]";
	}

	group.append("text")
		.attr("id", "colourlabel")
		.attr("x", ((width - img_width) / 2 - 1.0 * rectWidth))
		.attr("y", 0.9 * height + 1.5 * emFontSize)
		.attr("font-family", "Inconsolata")
		.attr("font-size", 1.25 * emFontSize)
		.attr("text-anchor", "middle")
		.attr("stroke", "none")
		.attr("opacity", 0.8)
		.text(bunit);

	if (va_count == 1) {
		var elem = d3.select("#legend");

		if (displayLegend)
			elem.attr("opacity", 1);
		else
			elem.attr("opacity", 0);
	}
	else {
		for (let index = 1; index <= va_count; index++) {
			var elem = d3.select("#legend" + index);

			if (displayLegend)
				elem.attr("opacity", 1);
			else
				elem.attr("opacity", 0);
		}
	}
}

function get_slope_from_multiplier(value) {
	var xmin = 0.01;
	var xmax = 100.0;

	var pmin = 0.001;
	var pmax = 0.5;

	return pmin + (pmax - pmin) * (value - xmin) / (xmax - xmin);
}

function get_noise_sensitivity_from_multiplier(value) {
	var xmin = Math.log(0.01);
	var xmax = Math.log(100.0);

	return 100.0 * (Math.log(value) - xmin) / (xmax - xmin);
}

function get_noise_sensitivity(value) {
	var xmin = Math.log(0.01);
	var xmax = Math.log(100.0);
	var x = xmin + (xmax - xmin) / 100.0 * parseFloat(value);

	return Math.exp(x);
}

function get_noise_sensitivity_string(value, precision) {
	var x = get_noise_sensitivity(value);

	return 'x' + x.toFixed(precision);
}

function resizeMe() {
	clearTimeout(idleResize);

	idleResize = setTimeout(function () {
		location.reload();
	}, 250);
}

function beforePrint() {
	console.log('before printing...');

	window.onresize = null;
}

function afterPrint() {
	console.log('after printing...');

	window.onresize = resizeMe;
}

function localStorage_read_boolean(key, defVal) {
	if (localStorage.getItem(key) !== null) {
		var value = localStorage.getItem(key);

		if (value == "true")
			return true;

		if (value == "false")
			return false;
	}
	else
		return defVal;
}

function localStorage_read_number(key, defVal) {
	if (localStorage.getItem(key) === null)
		return defVal;
	else
		return parseFloat(localStorage.getItem(key));
}

function localStorage_read_string(key, defVal) {
	if (localStorage.getItem(key) === null)
		return defVal;
	else
		return localStorage.getItem(key);
}

function localStorage_write_boolean(key, value) {
	if (value)
		localStorage.setItem(key, "true");
	else
		localStorage.setItem(key, "false");
}


function transpose(m) { return zeroFill(m.reduce(function (m, r) { return Math.max(m, r.length) }, 0)).map(function (r, i) { return zeroFill(m.length).map(function (c, j) { return m[j][i] }) }) } function zeroFill(n) { return new Array(n + 1).join("0").split("").map(Number) };

function contour_surface() {
	//return contour_surface_marching_squares() ;
	return contour_surface_webworker();
};

function contour_surface_marching_squares() {
	if (va_count > 1 && !composite_view)
		return;

	has_contours = false;

	try {
		d3.select('#contourPlot').remove();
	}
	catch (e) { };

	var data = [];

	var imageCanvas = imageContainer[va_count - 1].imageCanvas;
	var imageDataCopy = imageContainer[va_count - 1].imageDataCopy;
	var image_bounding_dims = imageContainer[va_count - 1].image_bounding_dims;

	if (composite_view) {
		imageCanvas = compositeCanvas;
		imageDataCopy = compositeImageData.data;
	}

	let min_value = 255;
	let max_value = 0;

	//for(var h=0;h<image_bounding_dims.height;h++)
	for (var h = image_bounding_dims.height - 1; h >= 0; h--) {
		var row = [];

		var xcoord = image_bounding_dims.x1;
		var ycoord = image_bounding_dims.y1 + h;
		var pixel = 4 * (ycoord * imageCanvas.width + xcoord);

		for (var w = 0; w < image_bounding_dims.width; w++) {
			//var z = imageDataCopy[pixel];	    
			var r = imageDataCopy[pixel];
			var g = imageDataCopy[pixel + 1];
			var b = imageDataCopy[pixel + 2];
			var z = (r + g + b) / 3;
			pixel += 4;

			if (z < min_value)
				min_value = z;

			if (z > max_value)
				max_value = z;

			row.push(z);
		}

		data.push(row);
	}

	//console.log(data);

	//console.log("min_pixel:", min_pixel, "max_pixel:", max_pixel) ;
	console.log("min_value:", min_value, "max_value:", max_value);

	var contours = parseInt(document.getElementById('contour_lines').value) + 1;
	var step = (max_value - min_value) / contours;
	var zs = d3.range(min_value + step, max_value, step);

	console.log(zs);

	var isoBands = [];
	for (var i = 1; i < zs.length; i++) {
		var lowerBand = zs[i - 1];
		var upperBand = zs[i];

		var band = MarchingSquaresJS.isoBands(data, lowerBand, upperBand - lowerBand);
		console.log('band', band);
		isoBands.push({ "coords": band, "level": i, "val": zs[i] });
	}

	//console.log(isoBands);    

	//return ;

	var elem = d3.select("#image_rectangle");
	var width = parseFloat(elem.attr("width"));
	var height = parseFloat(elem.attr("height"));

	var x = d3.scaleLinear()
		.range([0, width - 1])
		.domain([0, data[0].length - 1]);

	var y = d3.scaleLinear()
		.range([height, 1])
		.domain([0, data.length - 1]);

	var colours = d3.scaleLinear()
		.domain([min_value, max_value])
		.range(["#fff", "red"]);

	d3.select("#BackgroundSVG").append("svg")
		.attr("id", "contourPlot")
		.attr("x", elem.attr("x"))
		.attr("y", elem.attr("y"))
		.attr("width", width)
		.attr("height", height)
		.selectAll("path")
		.data(isoBands)
		.enter().append("path")
		//.style("fill",function(d) { return colours(d.level);})
		.style("fill", "none")
		.style("stroke", "black")
		.attr("opacity", 0.5)
		.attr("d", function (d) {
			var p = "";
			d.coords.forEach(function (aa, i) {
				p += (d3.line()
					.x(function (dat) { return x(dat[0]); })
					.y(function (dat) { return y(dat[1]); })
					.curve(d3.curveLinear)
				)(aa) + "Z";
			});
			return p;
		});

	has_contours = true;
}

function contour_surface_webworker() {
	if (va_count > 1 && !composite_view)
		return;

	has_contours = false;

	try {
		d3.selectAll('#contourPlot').remove();
	}
	catch (e) { };

	var data = [];

	var imageCanvas = imageContainer[va_count - 1].imageCanvas;
	var imageFrame = imageContainer[va_count - 1].imageFrame;
	var image_bounding_dims = imageContainer[va_count - 1].image_bounding_dims;

	if (composite_view) {
		imageCanvas = compositeCanvas;
		imageDataCopy = compositeImageData.data;
	}

	let min_value = 255;
	let max_value = 0;

	if (composite_view)
		for (var h = image_bounding_dims.height - 1; h >= 0; h--) {
			var row = [];

			var xcoord = image_bounding_dims.x1;
			var ycoord = image_bounding_dims.y1 + h;
			var pixel = 4 * (ycoord * imageCanvas.width + xcoord);

			for (var w = 0; w < image_bounding_dims.width; w++) {
				var r = imageDataCopy[pixel];
				var g = imageDataCopy[pixel + 1];
				var b = imageDataCopy[pixel + 2];
				var z = (r + g + b) / 3;
				pixel += 4;

				if (z < min_value)
					min_value = z;

				if (z > max_value)
					max_value = z;

				row.push(z);
			}

			data.push(row);
		}
	else //use imageFrame
	{
		for (var h = image_bounding_dims.height - 1; h >= 0; h--) {
			var row = [];

			var xcoord = image_bounding_dims.x1;
			var ycoord = image_bounding_dims.y1 + h;
			var pixel = ycoord * imageFrame.stride + xcoord;

			for (var w = 0; w < image_bounding_dims.width; w++) {
				var z = imageFrame.bytes[pixel];
				pixel += 1;

				if (z < min_value)
					min_value = z;

				if (z > max_value)
					max_value = z;

				row.push(z);
			}

			data.push(row);
		};
	}

	//console.log(data);

	//console.log("min_pixel:", min_pixel, "max_pixel:", max_pixel) ;
	console.log("min_value:", min_value, "max_value:", max_value);

	var contours = parseInt(document.getElementById('contour_lines').value) + 1;
	var step = (max_value - min_value) / contours;
	var zs = d3.range(min_value + step, max_value, step);

	console.log("zs:", zs);

	var completed_levels = 0;
	//parallel isoBands    
	for (var i = 1; i < zs.length; i++) {
		var lowerBand = zs[i - 1];
		var upperBand = zs[i];

		var CRWORKER = new Worker('contour_worker.js' + '?' + encodeURIComponent(get_js_version()));

		CRWORKER.addEventListener('message', function (e) {
			//console.log('Worker said: ', e.data);
			completed_levels++;

			var isoBands = [];
			isoBands.push({ "coords": e.data, "level": i, "val": zs[i] });

			//plot the isoBands
			var elem = d3.select("#image_rectangle");
			var width = parseFloat(elem.attr("width"));
			var height = parseFloat(elem.attr("height"));

			var x = d3.scaleLinear()
				.range([0, width - 1])
				.domain([0, data[0].length - 1]);

			var y = d3.scaleLinear()
				.range([height, 1])
				.domain([0, data.length - 1]);

			var colours = d3.scaleLinear()
				.domain([min_value, max_value])
				.range(["#fff", "red"]);

			d3.select("#ContourSVG").append("svg")
				.attr("id", "contourPlot")
				.attr("x", elem.attr("x"))
				.attr("y", elem.attr("y"))
				.attr("width", width)
				.attr("height", height)
				.selectAll("path")
				.data(isoBands)
				.enter().append("path")
				//.style("fill",function(d) { return colours(d.level);})
				.style("fill", "none")
				.style("stroke", "black")
				.attr("opacity", 0.5)
				.attr("d", function (d) {
					var p = "";
					d.coords.forEach(function (aa, i) {
						p += (d3.line()
							.x(function (dat) { return x(dat[0]); })
							.y(function (dat) { return y(dat[1]); })
							.curve(d3.curveLinear)
						)(aa) + "Z";
					});
					return p;
				});

			has_contours = true;

			if (completed_levels == zs.length - 1)
				hide_hourglass();
		}, false);

		//CRWORKER.postMessage('Hello World'); // Send data to our worker.    
		CRWORKER.postMessage({ data: data, level: i, lowerBand: lowerBand, upperBand: upperBand });
		//CRWORKER.postMessage({'cmd':'do some work'}) ;    
	};

	has_contours = true;
}

function test_webgl_support() {
	try {
		var canvas = document.createElement('canvas');
		return !!window.WebGLRenderingContext && (
			canvas.getContext('webgl') || canvas.getContext('experimental-webgl'));
	} catch (e) { return false; }
};

function enable_3d_view() {
	has_webgl = false;

	if (test_webgl_support()) {
		console.log("WebGL supported");

		/*(function () {
			var po = document.createElement('script'); po.type = 'text/javascript'; po.async = false;
			po.src = 'three.min.js' + '?' + encodeURIComponent(get_js_version());
			var s = document.getElementsByTagName('script')[0]; s.parentNode.insertBefore(po, s);
		})();

		(function () {
			var po = document.createElement('script'); po.type = 'text/javascript'; po.async = false;
			po.src = 'Detector.js' + '?' + encodeURIComponent(get_js_version());
			var s = document.getElementsByTagName('script')[0]; s.parentNode.insertBefore(po, s);
		})();

		(function () {
			var po = document.createElement('script'); po.type = 'text/javascript'; po.async = false;
			po.src = 'threex.keyboardstate.js' + '?' + encodeURIComponent(get_js_version());
			var s = document.getElementsByTagName('script')[0]; s.parentNode.insertBefore(po, s);
		})();

		(function () {
			var po = document.createElement('script'); po.type = 'text/javascript'; po.async = false;
			po.src = 'threex.windowresize.js' + '?' + encodeURIComponent(get_js_version());
			var s = document.getElementsByTagName('script')[0]; s.parentNode.insertBefore(po, s);
		})();

		(function () {
			var po = document.createElement('script'); po.type = 'text/javascript'; po.async = false;
			po.src = 'THREEx.FullScreen.js' + '?' + encodeURIComponent(get_js_version());
			var s = document.getElementsByTagName('script')[0]; s.parentNode.insertBefore(po, s);
		})();

		(function () {
			var po = document.createElement('script'); po.type = 'text/javascript'; po.async = false;
			po.src = 'TrackballControls.js' + '?' + encodeURIComponent(get_js_version());
			var s = document.getElementsByTagName('script')[0]; s.parentNode.insertBefore(po, s);
		})();*/

		(function () {
			var po = document.createElement('script'); po.type = 'text/javascript'; po.async = false;
			po.src = 'https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/three.min.js';
			var s = document.getElementsByTagName('script')[0]; s.parentNode.insertBefore(po, s);
		})();

		(function () {
			var po = document.createElement('script'); po.type = 'text/javascript'; po.async = false;
			po.src = 'https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/Detector.min.js';
			var s = document.getElementsByTagName('script')[0]; s.parentNode.insertBefore(po, s);
		})();

		(function () {
			var po = document.createElement('script'); po.type = 'text/javascript'; po.async = false;
			po.src = 'https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/threex.keyboardstate.min.js';
			var s = document.getElementsByTagName('script')[0]; s.parentNode.insertBefore(po, s);
		})();

		(function () {
			var po = document.createElement('script'); po.type = 'text/javascript'; po.async = false;
			po.src = 'https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/threex.windowresize.min.js';
			var s = document.getElementsByTagName('script')[0]; s.parentNode.insertBefore(po, s);
		})();

		(function () {
			var po = document.createElement('script'); po.type = 'text/javascript'; po.async = false;
			po.src = 'https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/THREEx.FullScreen.min.js';
			var s = document.getElementsByTagName('script')[0]; s.parentNode.insertBefore(po, s);
		})();

		(function () {
			var po = document.createElement('script'); po.type = 'text/javascript'; po.async = false;
			po.src = 'https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/TrackballControls.min.js';
			var s = document.getElementsByTagName('script')[0]; s.parentNode.insertBefore(po, s);
		})();

		(function () {
			var po = document.createElement('script'); po.type = 'text/javascript'; po.async = false;
			po.src = 'surface2.js' + '?' + encodeURIComponent(get_js_version());
			var s = document.getElementsByTagName('script')[0]; s.parentNode.insertBefore(po, s);
		})();

		has_webgl = true;
	}
	else
		console.log("WebGL not supported by your browser");
}

/*function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async*/ function mainRenderer() {
	try {
		enable_3d_view();
	}
	catch (e) {
		has_webgl = false;
		console.log('WebGL disabled', e);
	}

	//intercept print events
	if (window.matchMedia) {
		var mediaQueryList = window.matchMedia('print');
		mediaQueryList.addListener(function (mql) {
			if (mql.matches) {
				beforePrint();
			} else {
				afterPrint();
			}
		});
	}

	window.onbeforeprint = beforePrint;
	window.onafterprint = afterPrint;
	//end-of-printing

	if (votable.getAttribute('data-root-path') != null)
		ROOT_PATH = votable.getAttribute('data-root-path').trim();
	console.log("ROOT_PATH=" + ROOT_PATH);

	isLocal = (votable.getAttribute('data-server-mode').indexOf("LOCAL") > -1) ? true : false;

	endianness = getEndianness();
	console.log('endianness: ', endianness);

	if (localStorage.getItem("ui_theme") === null) {
		//if(isLocal)
		{
			/*theme = "bright" ;	
			colourmap = "haxby" ;*/

			theme = "dark";
			colourmap = "green";
		}
		/*else
		{
			theme = "dark" ;
			colourmap = "green" ;
		}*/

		localStorage.setItem("ui_theme", theme);
		localStorage.setItem("colourmap", colourmap);
	}
	else
		theme = localStorage.getItem("ui_theme");

	noise_sensitivity = 50; //get_noise_sensitivity(localStorage.getItem("noise_sensitivity")) ;

	if (localStorage.getItem("zoom_shape") === null) {
		zoom_shape = "circle";
		localStorage.setItem("zoom_shape", zoom_shape);
	}
	else
		zoom_shape = localStorage.getItem("zoom_shape");

	if (localStorage.getItem("intensity_mode") === null) {
		console.log("URL parameters:", window.location.search);

		//an override for FUGIN
		if (window.location.search.indexOf('fugin') > 0)
			intensity_mode = "mean";
		else
			intensity_mode = "integrated";

		localStorage.setItem("intensity_mode", intensity_mode);
	}
	else
		intensity_mode = localStorage.getItem("intensity_mode");

	if (localStorage.getItem("colourmap") === null) {
		if (theme == 'bright')
			colourmap = "haxby";
		else
			colourmap = "green";

		localStorage.setItem("colourmap", colourmap);
	}
	else
		colourmap = localStorage.getItem("colourmap");

	if (colourmap === null)
		colourmap = "green";

	//add a colourmap URL override
	let pos = window.location.search.indexOf('colourmap=');
	if (pos > 0) {
		//extract the colourmap parameter
		let params = window.location.search.substr(pos);
		console.log("colourmap parameters:", params);

		var result = {};
		params.split("&").forEach(function (part) {
			var item = part.split("=");
			result[item[0]] = decodeURIComponent(item[1]);
		});

		var tmp = result["colourmap"];
		if (tmp !== undefined)
			colourmap = tmp;

		console.log("colourmap:", result["colourmap"]);
	}

	if (localStorage.getItem("video_fps_control") === null) {
		video_fps_control = "auto";
		localStorage.setItem("video_fps_control", video_fps_control);
	}
	else
		video_fps_control = localStorage.getItem("video_fps_control");

	composite_view = (parseInt(votable.getAttribute('data-composite')) == 1) ? true : false;
	console.log("composite view:", composite_view);

	optical_view = false;

	if (firstTime) {
		fps = 60;//target fps; 60 is OK in Chrome but a bit laggish in Firefox
		fpsInterval = 1000 / fps;

		has_frequency_info = false;
		has_velocity_info = false;
		frame_multiplier = 1;

		imageData = null;
		newImageData = null;
		initKalmanFilter = false;
		windowLeft = false;
		streaming = false;
		video_playback = false;
		video_offset = null;
		video_timeout = -1;
		mol_pos = -1;
		idleMouse = -1;
		idleVideo = -1;
		moving = false;
		freqdrag = false;
		data_band_lo = 0;
		data_band_hi = 0;
		latency = 0;
		ping_latency = 0;
		computed = 0;
		processed = 0;
		cpuTime = 0;

		//image
		recv_seq_id = 0;
		sent_seq_id = 0;
		last_seq_id = 0;

		//video
		if (video_fps_control == 'auto')
			vidFPS = 5;//10
		else
			vidFPS = parseInt(video_fps_control);

		vidInterval = 1000 / vidFPS;

		//track the bitrate with a Kalman Filter
		target_bitrate = 1000;
		bitrate = target_bitrate;
		eta = 0.1;
		variance = 0.0;

		recv_vid_id = 0;
		sent_vid_id = 0;
		last_vid_id = 0;
		videoFrame = [];

		spectrum_stack = [];
		image_stack = [];
		video_stack = [];
		viewport_zoom_settings = null;
		zoom_dims = null;
		zoom_location = 'lower';
		zoom_scale = 25;
		xradec = null;
		molecules = [];

		tmp_data_min = 0;
		tmp_data_max = 0;

		user_data_min = null;
		user_data_max = null;

		freq_mouse_start = 0;
		freqdrag = false;
		session_freq_start = 0;
		session_freq_end = 0;
		session_frame_start = 0;
		session_frame_end = 0;
		frame_start = 0;
		frame_end = 0;

		mousedown = false;
		begin_x = 0;
		begin_y = 0;
		end_x = 0;
		end_y = 0;

		coordsFmt = localStorage_read_string("coordsFmt", "HMS");//'DMS' or 'HMS'

		//navigation = localStorage_read_string("navigation", "dynamic");//'dynamic' (classic) or 'static' (new)
		navigation = "dynamic";
		if (navigation == "static")
			zoom_scale = 1;

		displayCDMS = localStorage_read_boolean("displayCDMS", false);
		displayJPL = localStorage_read_boolean("displayJPL", false);
		displayRecomb = localStorage_read_boolean("displayRecomb", true);
		displayTopModel = localStorage_read_boolean("displayTopModel", false);
		displaySLAIM = localStorage_read_boolean("displaySLAIM", false);
		displayLovas = localStorage_read_boolean("displayLovas", true);
		displayToyaMA = localStorage_read_boolean("displayToyaMA", false);
		displayOSU = localStorage_read_boolean("displayOSU", false);
		displayIntensity = localStorage_read_number("displayIntensity", -1);

		realtime_spectrum = localStorage_read_boolean("realtime_spectrum", true);
		realtime_video = localStorage_read_boolean("realtime_video", true);
		experimental = localStorage_read_boolean("experimental", false);
		displayDownloadConfirmation = localStorage_read_boolean("displayDownloadConfirmation", true);
		welcome = localStorage_read_boolean("welcome_v4", true);

		autoscale = true;
		displayScalingHelp = localStorage_read_boolean("displayScalingHelp", true);
		last_spectrum = null;

		displayContours = false;
		displayLegend = localStorage_read_boolean("displayLegend", true);
		displayMolecules = localStorage_read_boolean("displayMolecules", true);
		displaySpectrum = localStorage_read_boolean("displaySpectrum", true);
		displayGridlines = localStorage_read_boolean("displayGridlines", false);
		displayBeam = false;

		has_contours = false;
		has_preferences = false;

		d3.select("body").append("div")
			.attr("id", "mainDiv")
			.attr("class", "main");

		if (theme == 'bright') {
			d3.select("body")
				.style('background-color', 'white')
				.style('color', 'black');

			d3.select("html")
				.style('background-color', 'white')
				.style('color', 'black');

			try {
				for (let i = 0; i < document.styleSheets.length; i++) {
					if (document.styleSheets[i].href != null)
						if (document.styleSheets[i].href.indexOf('fitswebql.css') > 0) {
							let stylesheet = document.styleSheets[i];
							console.log(document.styleSheets[i]);

							if (stylesheet.cssRules) {
								for (let j = 0; j < stylesheet.cssRules.length; j++)
									if (stylesheet.cssRules[j].selectorText === '.modal-content')
										stylesheet.deleteRule(j);

								for (let j = 0; j < stylesheet.cssRules.length; j++)
									if (stylesheet.cssRules[j].selectorText === '.list-group-item')
										stylesheet.deleteRule(j);
							}

							console.log(document.styleSheets[i]);
						}
				}
			}
			catch (e) {
				console.log('safely ignoring error:', e);
			}
		}

		votable = document.getElementById('votable');
		va_count = parseInt(votable.getAttribute('data-va_count'));
		datasetId = votable.getAttribute('data-datasetId');//make it a global variable	

		spectrum_stack = new Array(va_count);
		spectrum_scale = new Array(va_count);
		videoFrame = new Array(va_count);
		video_stack = new Array(va_count);

		for (let i = 0; i < va_count; i++) {
			spectrum_stack[i] = [];
			spectrum_scale[i] = 1;
			video_stack[i] = [];
			videoFrame[i] = null;
		};

		if (va_count > 1) {
			datasetId = [];

			for (let i = 0; i < va_count; i++)
				datasetId.push(votable.getAttribute('data-datasetId' + (i + 1)));

			console.log('LINE GROUP:', datasetId);

			//datasetId = votable.getAttribute('data-datasetId1') ;

			if (!composite_view)
				zoom_scale = 1;
		}

		var rect = document.getElementById('mainDiv').getBoundingClientRect();

		//set the default font-size (1em)		
		//emFontSize = Math.max(12, 0.011 * (0.2 * rect.width + 0.8 * rect.height));
		emFontSize = Math.max(12, 0.011 * (0.2 * rect.width + 0.8 * rect.height));
		emStrokeWidth = Math.max(1, 0.1 * emFontSize);
		document.body.style.fontSize = emFontSize + "px";
		console.log("emFontSize : ", emFontSize.toFixed(2), "emStrokeWidth : ", emStrokeWidth.toFixed(2));

		var width = rect.width - 20;
		var height = rect.height - 20;

		d3.select("#mainDiv").append("canvas")
			.attr("id", "BackHTMLCanvas")
			.attr("width", width)
			.attr("height", height)
			.attr('style', 'position: fixed; left: 10px; top: 10px; z-index: 0');

		if (va_count > 1) {
			for (let index = 0; index < va_count; index++) {
				d3.select("#mainDiv").append("canvas")
					.attr("id", "HTMLCanvas" + (index + 1))
					.attr("width", width)
					.attr("height", height)
					.attr('style', 'position: fixed; left: 10px; top: 10px; z-index: ' + (index + 1));
			}
		}

		d3.select("#mainDiv").append("canvas")
			.attr("id", "HTMLCanvas")
			.attr("width", width)
			.attr("height", height)
			.attr('style', 'position: fixed; left: 10px; top: 10px; z-index: ' + (va_count + 1));

		d3.select("#mainDiv").append("canvas")
			.attr("id", "VideoCanvas")
			.attr("width", width)
			.attr("height", height)
			.attr('style', 'position: fixed; left: 10px; top: 10px; z-index: 49');

		d3.select("#mainDiv").append("canvas")
			.attr("id", "CompositeCanvas")
			.attr("width", width)
			.attr("height", height)
			.attr('style', 'position: fixed; left: 10px; top: 10px; z-index: 50');

		d3.select("#mainDiv").append("svg")
			.attr("id", "ContourSVG")
			.attr("width", width)
			.attr("height", height)
			.attr('style', 'position: fixed; left: 10px; top: 10px; z-index: 51');

		d3.select("#mainDiv").append("svg")
			.attr("id", "BackgroundSVG")
			.attr("width", width)
			.attr("height", height)
			.attr('style', 'position: fixed; left: 10px; top: 10px; z-index: 52');

		d3.select("#mainDiv").append("canvas")
			.attr("id", "ZOOMCanvas")
			.attr("width", width)
			.attr("height", height)
			.attr('style', 'position: fixed; left: 10px; top: 10px; z-index: 53');

		d3.select("#mainDiv").append("svg")
			.attr("id", "BackSVG")
			.attr("width", width)
			.attr("height", height)
			.attr('style', 'position: fixed; left: 10px; top: 10px; z-index: 54; cursor: default; mix-blend-mode: none');//difference or lighten or screen //other than none causes problems with an older Firefox v45

		//spectrum
		var blend = '';

		if (theme == 'bright')
			blend = 'mix-blend-mode: difference; ';

		d3.select("#mainDiv").append("canvas")
			.attr("id", "SpectrumCanvas")
			.attr("width", width)
			.attr("height", height)
			.attr('style', blend + 'position: fixed; left: 10px; top: 10px; z-index: 54');

		d3.select("#mainDiv").append("svg")
			.attr("id", "FrontSVG")
			.attr("width", width)
			.attr("height", height)
			.on("mouseenter", hide_navigation_bar)
			.attr('style', 'position: fixed; left: 10px; top: 10px; z-index: 55; cursor: default');

		d3.select("#BackSVG").append("svg:image")
			.attr("id", "jvoLogo")
			.attr("x", (width - 1 - 199))
			.attr("y", (height - 1 - 109))
			.attr("xlink:href", "http://jvo.nao.ac.jp/images/JVO_logo_199x109.png")
			.attr("width", 199)
			.attr("height", 109)
			.attr("opacity", 0.5);

		var has_fits = votable.getAttribute('data-has-fits');
		var display_progress = 'block';
		if (has_fits == 'true')
			display_progress = 'none';

		var div = d3.select("body").append("div")
			.attr("id", "welcome")
			.attr("class", "container welcome")
			.style('display', display_progress);

		var group = div.append("g")
			.attr("id", "welcomeGroup");

		group.append("h1")
			.text("FITSWebQL v4");

		group.append("p")
			.text(votable.getAttribute('data-server-version') + "/" + get_js_version());

		group.append("p")
			.text("Server is processing your request. Please wait...");

		if (va_count == 1) {
			/*group.append("h4")
			.append("p")
			.text(datasetId) ;*/

			group.append("div")
				.attr("class", "progress")
				.append("div")
				.attr("id", "progress-bar" + va_count)
				.attr("class", "progress-bar progress-bar-info progress-bar-striped active")
				.attr("role", "progressbar")
				.attr("aria-valuenow", 0)
				.attr("aria-valuemin", 0)
				.attr("aria-valuemax", 100)
				.style("width", "0%")
				.html("0%");
		}
		else {
			for (let index = 0; index < va_count; index++) {
				/*group.append("h4")
					.append("p")
					.text(datasetId[index]) ;*/

				group.append("div")
					.attr("class", "progress")
					.append("div")
					.attr("id", "progress-bar" + (index + 1))
					.attr("class", "progress-bar progress-bar-info progress-bar-striped active")
					.attr("role", "progressbar")
					.attr("aria-valuenow", 0)
					.attr("aria-valuemin", 0)
					.attr("aria-valuemax", 100)
					.style("width", "0%")
					.html("0%");
			}
		}

		d3.select("body").append("div")
			.attr("id", "molecularlist")
			.attr("class", "molecularmodal");

		display_range_validation();

		display_menu();

		setup_help();

		setup_FITS_header_page();

		if (welcome)
			show_welcome();

		display_hourglass();

		if (!isLocal && va_count == 1)
			fetch_binned_image(datasetId + '_00_00_00');

		compositeCanvas = null;
		compositeImageData = null;
		compositeViewportCanvas = null;
		compositeViewportImageData = null;

		fitsContainer = new Array(va_count);
		imageContainer = new Array(va_count);
		mean_spectrumContainer = new Array(va_count);
		integrated_spectrumContainer = new Array(va_count);
		wsConn = new Array(va_count);

		notifications_received = new Array(va_count);
		previous_progress = new Array(va_count);
		notifications_completed = 0;

		for (let i = 0; i < va_count; i++) {
			fitsContainer[i] = null;
			mean_spectrumContainer[i] = null;
			integrated_spectrumContainer[i] = null;
			imageContainer[i] = null;
			wsConn[i] = null;

			notifications_received[i] = 0;
			previous_progress[i] = -1;
		}

		image_count = 0;
		viewport_count = 0;
		spectrum_count = 0;

		if (va_count == 1) {
			open_websocket_connection(datasetId, 1);

			fetch_image(datasetId, 1, false);

			fetch_spectrum(datasetId, 1, false);

			fetch_spectral_lines(datasetId, 0, 0);
		}
		else {
			for (let index = 1; index <= va_count; index++) {
				console.log(index, datasetId.rotate(index - 1));

				open_websocket_connection(datasetId.rotate(index - 1).join(";"), index);

				fetch_image(datasetId[index - 1], index, false);

				fetch_spectrum(datasetId[index - 1], index, false);

				//sleep(1000) ;
			}
		}

	}

	firstTime = false;
}
