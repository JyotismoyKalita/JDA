import './Tab.css'

function Tab({caption, isSelected, counts}){
    return (
        <div className={`Tab ${isSelected ? 'Tab-Active' : ''}`}>
            <span className="Tab-Caption">{caption}</span>
            {counts[caption] > 0 && (
                <div className='Tab-Counter'>{counts[caption]}</div>
            )}
        </div>
    );
}

export default Tab;