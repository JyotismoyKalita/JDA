import './Tab.css'
import '../theme/colors.css'

function Tab({caption, isSelected, counts}){
    return (
        <div className='Tab Primary'>
            <div className='Tab-Caption'>
                {caption}
                {(counts[caption]>0) && <div className='Tab-Counter BG-Quarternary'>{counts[caption]}</div>}
            </div> 
            <div className={isSelected ? "Tab-Select BG-Quarternary" : "Tab-Select"}></div>
        </div>
    );
}

export default Tab;