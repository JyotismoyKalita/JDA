import './Progress.css'

function Progress({connections, total}){
    const chunkSize = total / connections.length;
    return (
        <div className='Progress-Cover BG-Secondary'>  
            {connections.map((element, index) => {
              const val = `${element * 100  / chunkSize}%`;
              return (
                <div key={index} className='Progress-Div'>
                    <div className='Progress-Fill BG-Quarternary' style={{width: val}}></div>
                </div>
              );  
            })}

        </div>
    );
}

export default Progress;