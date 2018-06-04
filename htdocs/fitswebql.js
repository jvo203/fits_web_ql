function goto_url(url)
{
    window.location.href = url;
}

function localStorage_read_lastdir(key)
{
    if (localStorage.getItem(key) === null)
	return "" ;
    else
	return localStorage.getItem(key) ;
}

function compare_files(a,b)
{
	if (a.name < b.name)
	  return -1;

	if (a.name > b.name)
	  return 1;

	return 0;
  }

function show_directory_contents(response)
{
 	$("#filesystem").remove();
    $("#container").append($("<div></div>")
		     .attr("id", "filesystem")) ;
    
    let loc = response.location ;
    let dirs = loc.split('/') ;	    	   	        
    
    $("#filesystem").append($("<ul></ul>")
			    .attr("id", "breadcrumb")
			    //.css("position", "fixed")
			    .attr("class", "breadcrumb"));    

    if(theme == 'bright')
	$("#breadcrumb").css('background-color', 'darkgrey') ;
    
    /*$(window).scroll(function(){
	$("#breadcrumb").css({"margin-top": ($(window).scrollTop()) + "px", "margin-left":($(window).scrollLeft()) + "px"});
    });*/
    
    //navigation
    var dir = "" ;
    for(let i=0;i<dirs.length;i++)
    {
	if(dirs[i] != "")
	    dir += "/" + dirs[i] ;

	var cmd = "fetch_directory(\"" + dir + "\")" ;		
	
	$("#breadcrumb").append($("<li></li>")
				.attr("class", "breadcrumb-item")
				.append($("<a></a>")					
					.attr("onclick", cmd)
					.css("cursor", "pointer")
					.text(dirs[i])));	
    }
    
    $('#breadcrumb').append($("<a></a>")
			    .attr("class", "btn btn-link")
			    .attr("onclick", "fetch_directory(\"\")")
			    .css("float", "right")
			    .html("<span class=\"glyphicon glyphicon-home\"></span><span style='font-size: 1.0em; padding: 0.5em'>HOME</span>"));    
    
    $("#filesystem").append($("<table></table>")
			    .attr("id", "files")
			    .attr("class", "table table-hover")
			    .html("<thead><tr style=\"color:inherit\"><th>name</th><th>size</th><th>last modified</th></tr></thead>")) ;
    //class=\"danger\" style=\"color:black\"
    
    //contents
    filelist = response.contents ;
	
	//sort files/dirs alphabetically by name
	filelist.sort(compare_files);

    $("#files").append($("<tbody></tbody>")
		       .attr("id", "tbody")) ;

    //add go up one step
    if(loc != "/")
    {
	dir = "" ;
	for(let i=0;i<dirs.length-1;i++)
	{
	    if(dirs[i] != "")
		dir += "/" + dirs[i] ;
	}

	if(dir == "")
	    dir = "/" ;
	
	var cmd = "fetch_directory(\"" + dir + "\")" ;
	
	$("#tbody").append($("<tr></tr>")
			   .css("cursor", "pointer")
			   .attr("onclick", cmd)
			   .html("<td><span class=\"glyphicon glyphicon-level-up\"></span>&nbsp;&nbsp;" + ".." + "</td><td></td><td></td>"));
    }
    
    //list directories first
    for(let i=0;i<filelist.length;i++)
    {
	if(filelist[i].type == "dir")
	{
	    var cmd ;

	    if(loc == "/")
		cmd = "fetch_directory('" + loc + filelist[i].name.replace(/'/g, "\\'") + "')" ;
	    else
		cmd = "fetch_directory('" + loc + '/' + filelist[i].name.replace(/'/g, "\\'") + "')" ;			    	    
	    
	    //class=\"text-right\"
	    $("#tbody").append($("<tr></tr>")
			       .css("cursor", "pointer")
			       .attr("onclick", cmd)
			       .html("<td><span class=\"glyphicon glyphicon-folder-open\"></span>&nbsp;&nbsp;" + filelist[i].name + "</td><td></td><td>" + filelist[i].last_modified + "</td>"));
	}
    }

    //then files
    for(let i=0;i<filelist.length;i++)
    {	
	if(filelist[i].type == "file")
	{
	    var path = loc ;
	    var filename = filelist[i].name ;

	    var name = filename.substr(0, filename.lastIndexOf('.'));
	    var ext = filename.substr(filename.lastIndexOf('.')+1);
	    var url = "/fitswebql/FITSWebQL.html?dir=" + encodeURIComponent(path) + "&ext=" + encodeURIComponent(ext);
	    
	    var line_group = find_line_group(filename) ;
	    var line_group_str = null ;
	    var composite = false ;

	    if(filename.indexOf("FGN_") > -1 && filename.indexOf("cube.fits") > -1)
		composite = true ;
	    
	    if(line_group.length > 0)
	    {
		line_group_str = 'LINE GROUP:' ;
		
		for(let i=0;i<line_group.length;i++)
		{
		    let filename = line_group[i] ;
		    let name = filename.substr(0, filename.lastIndexOf('.'));		    
		    
		    line_group_str += '\n' + filename ;		    		    
		    url += "&filename" + (i+1) + "=" + encodeURIComponent(name) ;
		}

		if(composite)		    
		    url += "&view=composite" ;
	    }
	    else
		url += "&filename=" + encodeURIComponent(name) ;

	    //enforce a tone mapping
	    url += "&flux=logistic" ;
	    
	    //single-file URL
	    //var url = "/fitswebql/FITSWebQL.html?dir=" + encodeURIComponent(path) + "&ext=" + encodeURIComponent(ext) + "&filename=" + encodeURIComponent(name) ;
	    
	    var tmp = "goto_url('" + url.replace(/'/g, "\\'") + "')" ;	    
	    //var cmd = "find_line_group('" + filelist[i].name.replace(/'/g, "\\'") + "')" ;	    	    
	    
	    //style=\"color: inherit\"
	    $("#tbody").append($("<tr></tr>")
			       .css("cursor", "pointer")
			       //.css("color", "black")
			       //.attr("class", "danger")
			       .attr("onclick", tmp)
			       //.attr("onmouseenter", cmd)
			       .attr('title', line_group_str)
			       .html("<td><p href=\"" + url + "\"><span class=\"glyphicon glyphicon-open-file\"></span>&nbsp;&nbsp;" + filelist[i].name + "</p></td><td>" + numeral(filelist[i].size).format('0.0 b') + "</td><td>" + filelist[i].last_modified + "</td>"));
	}
    }

    $("#filesystem").append($("<br></br>"));    
    
    $("body").css("cursor", "default") ;
}

function fetch_directory(dir)
{
    $("body").css("cursor", "wait") ;
    
    var xmlhttp = new XMLHttpRequest();    

    var url = 'get_directory' ;

    if(dir != "")
	url += '?dir=' + encodeURIComponent(dir) ;     
    
    xmlhttp.onreadystatechange = function() {
	if (xmlhttp.readyState == 4 && xmlhttp.status == 200)
	{
	    //console.log(xmlhttp.responseText) ;
	    
	    let response = JSON.parse(xmlhttp.responseText);

	    show_directory_contents(response) ;

	    localStorage.setItem("lastdir", dir) ;
	}
    }
    
    xmlhttp.open("GET", url, true);
    xmlhttp.timeout = 0 ;
    xmlhttp.send();
}

function escapeRegExp(str)
{
  return str.replace(/[\-\[\]\/\{\}\(\)\*\+\?\.\\\^\$\|]/g, "\\$&");
}

function find_line_group(name)
{
    if(filelist == null)
	return [] ;

    console.log('find_line_group:', name) ;

    var matches = [] ;
    
    var pos = name.indexOf('_v') ;

    if(pos > 0)
    {
	let str = name.substring(0,pos) ;	

	pos2 = str.lastIndexOf('_') ;

	if(pos2 > -1)
	{
	    let line = str.substring(pos2+1) ;
	    //console.log('LINE:', line) ;

	    let prefix = str.substring(0, pos2+1) ;
	    let postfix = name.substring(pos) ;
	    //console.log(prefix,postfix) ;
	    
	    var patt = new RegExp(escapeRegExp(prefix)+'.*'+escapeRegExp(postfix)) ;	    
	    //console.log(patt) ;	    
	    
	    for(let i=0;i<filelist.length;i++)
	    {
		if(filelist[i].type == "file")
		{		    
		    if(patt.test(filelist[i].name))
			matches.push(filelist[i].name);
		}
	    }
	}

	console.log(matches) ;
    }
    
    return matches ;
}

function main()
{
    filelist = null ;
    
    if (localStorage.getItem("ui_theme") === null)
	theme = "dark" ;//default theme, needs to be aligned with the main FITSWebQL;  "dark" or "bright"
    else
	theme = localStorage.getItem("ui_theme") ;

    if(theme == 'bright')
    {		
	$("body").css('background-color', 'white') ;
	$("body").css('color', 'black') ;

	try {
	    for(let i=0;i<document.styleSheets.length;i++)
		if(document.styleSheets[i].href.indexOf('fitswebql.css') > 0)
	    {
		let stylesheet = document.styleSheets[i] ;
		console.log(document.styleSheets[i]);

		if (stylesheet.cssRules)
		{
		    for (let j=0; j<stylesheet.cssRules.length; j++)
			if (stylesheet.cssRules[j].selectorText === '.modal-content')        	
			    stylesheet.deleteRule (j);		    		    
		}

		console.log(document.styleSheets[i]);
	    }
	}
	catch (e) {
	    console.log(e) ;
	}
    }
    
    //fetch the home directory first
    fetch_directory(localStorage_read_lastdir("lastdir")) ;
}
