const MS_PER_DAY = 86400000;

function updatePolygons(geojsonData) {
    currentPolygons.forEach(function(polygon) {
        polygon.remove();
    });
    currentPolygons = [];

    L.geoJSON(geojsonData, {
        pointToLayer: (feature, latlng) => {
            return new L.Circle(latlng, {radius: 500});
        },
        style: function(feature) {
            let start = Date.parse(feature.properties.start);
            let end = Date.parse(feature.properties.end);
            let beyond = end + MS_PER_DAY;
            let date = Date.now()
            let falloff = date - MS_PER_DAY * 30;

            if(end < date){
                return
            }
            if(date > beyond){
                return {
                    color: '#4444ff',
                    weight: 2,
                    opacity: 0.25
                }
            } else if(start <= date){
                return {
                    color: '#ff0000',
                    weight: 2,
                    opacity: 0.65
                }
            }
            return {
                color: '#ff6200',
                weight: 2,
                opacity: 0.25
            };
        },
        onEachFeature: function(feature, layer) {
            const start_date = new Date(feature.properties.start);
            const start_string = `${start_date.toLocaleDateString('en-AU', { dateStyle: 'full'})}`;
            const end_date = new Date(feature.properties.end);
            const end_string = `${end_date.toLocaleDateString('en-AU', { dateStyle: 'full' })}`;
            const duration = `${Math.floor((end_date - start_date) / MS_PER_DAY)} day(s)`;
            const time_until_start = Math.floor((start_date - Date.now()) / MS_PER_DAY);
            let start_notice = '';
            if(time_until_start > 0){
                start_notice = `<strong>Days until start:</strong> ${time_until_start}<br />`
            }
            const [title, posted] = feature.properties.title.split(" Dated ", 2);
            const popupContent = `
                                    <div class='custom-popup'>
                                        <p><strong>${title}</strong><br />Published ${posted}</p>
                                        <p><strong>Begins:</strong> ${start_string}<br/><strong>Ends:</strong> ${end_string}</p>
                                        <p>${start_notice}<strong>Duration:</strong> ${duration}</p>
                                        <p><a href='${feature.properties.uri}' target='_blank'>View Details</a></p>
                                        ${feature.properties.img_uri ?
                                        `<img alt='${feature.properties.title}' src='${feature.properties.img_uri}' style='max-width: 200px;'>`
                                            : ''}
                                        </div>
                                    `;
            layer.bindPopup(popupContent);
            currentPolygons.push(layer);
        }
    }).addTo(map);
}
